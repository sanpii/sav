#![warn(warnings)]

mod database;
mod error;
mod expense;

use database::Database;
use error::Error;
use std::collections::HashMap;

type Flash = rocket::response::Flash<rocket::response::Redirect>;
type Result<T = ()> = std::result::Result<T, crate::Error>;
type Response = rocket_dyn_templates::Template;

#[derive(Debug, Default, rocket::FromForm)]
struct FormData<'r> {
    pub id: Option<i32>,
    pub created_at: String,
    pub serial: Option<String>,
    pub name: String,
    pub url: Option<String>,
    pub shop: String,
    pub warranty: i32,
    pub price: f32,
    pub photo: Option<rocket::fs::TempFile<'r>>,
    pub invoice: Option<rocket::fs::TempFile<'r>>,
    pub notice: Option<rocket::fs::TempFile<'r>>,
}

impl FormData<'_> {
    fn parse_date(date: &str) -> chrono::NaiveDateTime {
        chrono::NaiveDate::parse_from_str(date, "%F")
            .unwrap()
            .and_time(chrono::NaiveTime::default())
    }
}

#[rocket::launch]
async fn launch() -> _ {
    rocket::build()
        .attach(Database::fairing())
        .attach(rocket::fairing::AdHoc::on_ignite(
            "data_dir config",
            |rocket| {
                Box::pin(async {
                    let data_dir = rocket
                        .figment()
                        .find_value("data_dir")
                        .unwrap()
                        .as_str()
                        .unwrap_or("data/")
                        .to_string();

                    rocket.manage(DataDir(data_dir))
                })
            },
        ))
        .attach(rocket_dyn_templates::Template::custom(|engines| {
            engines
                .tera
                .register_function("has_media", Box::new(has_media));
            engines
                .tera
                .register_function("pager", elephantry_extras::tera::Pager);
        }))
        .mount(
            "/static",
            rocket::fs::FileServer::from(concat!(env!("CARGO_MANIFEST_DIR"), "/static")),
        )
        .mount(
            "/",
            rocket::routes![
                index, add, create, edit, save, delete, trash, untrash, photo, invoice, notice,
            ],
        )
}

#[derive(rocket::FromForm)]
struct Params {
    page: Option<usize>,
    limit: Option<usize>,
    trashed: Option<bool>,
    q: Option<String>,
}

struct DataDir(String);

#[rocket::get("/?<params..>")]
async fn index(
    database: Database,
    data_dir: &rocket::State<DataDir>,
    params: Params,
    flash: Option<rocket::request::FlashMessage<'_>>,
) -> Result<Response> {
    let pagination = elephantry_extras::Pagination {
        page: params.page.unwrap_or(1),
        limit: params.limit.unwrap_or(50),
    };
    let trashed = params.trashed.unwrap_or(false);

    let pager = database.all(params.q.clone(), pagination, trashed).await?;

    let base_url = if let Some(q) = &params.q {
        format!("/?q={q}")
    } else {
        String::new()
    };

    let mut context = tera::Context::new();
    context.insert("base_url", &base_url);
    context.insert("data_dir", &data_dir.0);
    context.insert("pager", &pager);
    context.insert("q", &params.q);
    context.insert(
        "flash",
        &flash.map(|x| (x.kind().to_string(), x.message().to_string())),
    );

    let template = rocket_dyn_templates::Template::render("expense/list", context.into_json());

    Ok(template)
}

#[rocket::get("/expenses/add")]
async fn add(database: Database) -> Result<Response> {
    let mut context = tera::Context::new();
    context.insert("shops", &database.shops().await?);
    let template = rocket_dyn_templates::Template::render("expense/edit", context.into_json());

    Ok(template)
}

#[rocket::post("/expenses/add", data = "<form_data>")]
async fn create(
    database: Database,
    data_dir: &rocket::State<DataDir>,
    form_data: rocket::form::Form<FormData<'_>>,
) -> Result<Flash> {
    save(database, data_dir, -1, form_data).await
}

#[rocket::get("/expenses/<id>/edit")]
async fn edit(database: Database, id: i32) -> Result<Option<Response>> {
    let expense = match database.get(id).await? {
        Some(expense) => expense,
        None => return Ok(None),
    };
    let mut context = tera::Context::from_serialize(expense)?;
    context.insert("shops", &database.shops().await?);
    let template = rocket_dyn_templates::Template::render("expense/edit", context.into_json());

    Ok(Some(template))
}

#[rocket::post("/expenses/<id>/edit", data = "<form_data>")]
async fn save(
    database: Database,
    data_dir: &rocket::State<DataDir>,
    id: i32,
    mut form_data: rocket::form::Form<FormData<'_>>,
) -> Result<Flash> {
    let entity = expense::Entity::from(&form_data);

    let (entity, msg) = if id > 0 {
        (
            database.update(id, entity).await?.unwrap(),
            "Achat mis à jour",
        )
    } else {
        (database.create(entity).await?, "Achat créé")
    };

    if let Some(ref mut photo) = form_data.photo {
        write_file(&data_dir.0, "photo", photo, &entity).await?;
    }
    if let Some(ref mut invoice) = form_data.invoice {
        write_file(&data_dir.0, "invoice", invoice, &entity).await?;
    }
    if let Some(ref mut notice) = form_data.notice {
        write_file(&data_dir.0, "notice", notice, &entity).await?;
    }

    Ok(Flash::success(rocket::response::Redirect::to("/"), msg))
}

async fn write_file(
    data_dir: &str,
    file_type: &str,
    file: &mut rocket::fs::TempFile<'_>,
    expense: &crate::expense::Entity,
) -> Result {
    let path = media_path(data_dir, expense.id.unwrap(), file_type);

    if file.len() == 0 {
        return Ok(());
    }

    let dir = path.parent().unwrap();
    if !dir.exists() {
        std::fs::create_dir(dir)?;
    }

    file.move_copy_to(&path).await?;

    Ok(())
}

#[rocket::get("/expenses/<id>/delete")]
async fn delete(database: Database, data_dir: &rocket::State<DataDir>, id: i32) -> Result<Flash> {
    database.delete(id).await?;
    std::fs::remove_dir_all(media_path(&data_dir.0, id, "photo").parent().unwrap())?;

    Ok(Flash::success(
        rocket::response::Redirect::to("/"),
        "Achat supprimé",
    ))
}

#[rocket::get("/expenses/<id>/trash")]
async fn trash(database: Database, id: i32) -> Result<Flash> {
    database.trash(id).await?;

    Ok(Flash::success(
        rocket::response::Redirect::to("/"),
        "Achat jeté",
    ))
}

#[rocket::get("/expenses/<id>/untrash")]
async fn untrash(database: Database, id: i32) -> Result<Flash> {
    database.untrash(id).await?;

    Ok(Flash::success(
        rocket::response::Redirect::to("/"),
        "Achat recyclé",
    ))
}

#[rocket::get("/expenses/<id>/photo")]
async fn photo(data_dir: &rocket::State<DataDir>, id: i32) -> Option<rocket::fs::NamedFile> {
    media(&data_dir.0, id, "photo").await
}

#[rocket::get("/expenses/<id>/invoice")]
async fn invoice(data_dir: &rocket::State<DataDir>, id: i32) -> Option<rocket::fs::NamedFile> {
    media(&data_dir.0, id, "invoice").await
}

#[rocket::get("/expenses/<id>/notice")]
async fn notice(data_dir: &rocket::State<DataDir>, id: i32) -> Option<rocket::fs::NamedFile> {
    media(&data_dir.0, id, "notice").await
}

async fn media(data_dir: &str, id: i32, file_type: &str) -> Option<rocket::fs::NamedFile> {
    let path = media_path(data_dir, id, file_type);

    rocket::fs::NamedFile::open(path).await.ok()
}

fn media_path(data_dir: &str, id: i32, file_type: &str) -> std::path::PathBuf {
    let filename = format!("{data_dir}/{id}/{file_type}");

    std::path::PathBuf::from(&filename)
}

fn has_media(args: &HashMap<String, tera::Value>) -> tera::Result<tera::Value> {
    let data_dir = match args.get("data_dir") {
        Some(val) => tera::from_value::<String>(val.clone())?,
        None => return Err("oops".into()),
    };

    let id = match args.get("id") {
        Some(val) => tera::from_value::<i32>(val.clone())?,
        None => return Err("oops".into()),
    };

    let file_type = match args.get("file_type") {
        Some(val) => tera::from_value::<String>(val.clone())?,
        None => return Err("oops".into()),
    };

    let exists = media_path(&data_dir, id, &file_type).exists();
    let value = tera::to_value(exists)?;

    Ok(value)
}
