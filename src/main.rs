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

macro_rules! read {
    ($entry:ident -> String) => {{
        use std::io::Read;
        let mut t = String::new();
        $entry.data.read_to_string(&mut t).expect("not text");

        if t.is_empty() {
            None
        } else {
            Some(t)
        }
    }};

    ($entry:ident -> Vec) => {{
        use std::io::Read;
        let mut t = Vec::new();
        $entry.data.read_to_end(&mut t).expect("not text");

        if t.is_empty() {
            None
        } else {
            Some(t)
        }
    }};

    ($entry:ident -> $ty:ty) => {{
        read!($entry -> String).map(|x| x.parse().unwrap())
    }};
}

macro_rules! field {
    ($form_data:ident . $field:ident = $entry:ident as Option<$ty:ident>) => {{
        if &*$entry.headers.name == stringify!($field) {
            $form_data.$field = read!($entry -> $ty);
        }
    }};

    ($form_data:ident . $field:ident = $entry:ident as $ty:ident) => {{
        if &*$entry.headers.name == stringify!($field) {
            $form_data.$field = read!($entry -> $ty).unwrap();
        }
    }};
}

#[derive(Clone, Debug, Default)]
struct FormData {
    pub id: Option<i32>,
    pub created_at: String,
    pub serial: Option<String>,
    pub name: String,
    pub url: Option<String>,
    pub shop: String,
    pub warranty: i32,
    pub price: f32,
    pub photo: Option<Vec<u8>>,
    pub invoice: Option<Vec<u8>>,
    pub notice: Option<Vec<u8>>,
}

impl FormData {
    fn parse_date(date: &str) -> chrono::NaiveDateTime {
        chrono::NaiveDate::parse_from_str(date, "%F")
            .unwrap()
            .and_time(chrono::NaiveTime::default())
    }
}

#[rocket::async_trait]
impl<'r> rocket::data::FromData<'r> for FormData {
    type Error = crate::Error;

    async fn from_data(
        request: &'r rocket::Request<'_>,
        data: rocket::Data<'r>,
    ) -> rocket::data::Outcome<'r, Self> {
        let d = <&'r str>::from_data(request, data).await.unwrap();

        let ct = request
            .headers()
            .get_one("Content-Type")
            .expect("no content-type");
        let idx = ct.find("boundary=").expect("no boundary");
        let boundary = &ct[(idx + "boundary=".len())..];

        let mut mp = multipart::server::Multipart::with_body(d.as_bytes(), boundary);

        let mut form_data = FormData::default();

        mp.foreach_entry(|mut entry| {
            field!(form_data.id = entry as Option<i32>);
            field!(form_data.created_at = entry as String);
            field!(form_data.serial = entry as Option<String>);
            field!(form_data.name = entry as String);
            field!(form_data.url = entry as Option<String>);
            field!(form_data.shop = entry as String);
            field!(form_data.warranty = entry as i32);
            field!(form_data.price = entry as f32);
            field!(form_data.photo = entry as Option<Vec>);
            field!(form_data.invoice = entry as Option<Vec>);
            field!(form_data.notice = entry as Option<Vec>);
        })
        .expect("Unable to iterate");

        rocket::data::Outcome::Success(form_data)
    }
}

impl From<FormData> for expense::Entity {
    fn from(data: FormData) -> Self {
        Self {
            id: data.id,
            created_at: FormData::parse_date(&data.created_at),
            serial: data.serial.clone(),
            name: data.name.clone(),
            url: data.url.clone(),
            shop: data.shop.clone(),
            warranty: data.warranty,
            price: data.price,
            trashed_at: None,

            warranty_at: FormData::parse_date(&data.created_at),
            warranty_active: false,
            trashed: false,
        }
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
    let page = params.page.unwrap_or(1);
    let trashed = params.trashed.unwrap_or(false);
    let limit = params.limit.unwrap_or(50);

    let pager = database.all(params.q.clone(), page, limit, trashed).await?;

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
    form_data: FormData,
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
    form_data: FormData,
) -> Result<Flash> {
    let entity = form_data.clone().into();

    let (entity, msg) = if id > 0 {
        (
            database.update(id, entity).await?.unwrap(),
            "Achat mis à jour",
        )
    } else {
        (database.create(entity).await?, "Achat créé")
    };

    if let Some(photo) = &form_data.photo {
        write_file(&data_dir.0, "photo", photo, &entity)?;
    }
    if let Some(invoice) = &form_data.invoice {
        write_file(&data_dir.0, "invoice", invoice, &entity)?;
    }
    if let Some(notice) = &form_data.notice {
        write_file(&data_dir.0, "notice", notice, &entity)?;
    }

    Ok(Flash::success(rocket::response::Redirect::to("/"), msg))
}

fn write_file(
    data_dir: &str,
    file_type: &str,
    content: &[u8],
    expense: &crate::expense::Entity,
) -> Result {
    use std::io::Write;

    if content.is_empty() {
        return Ok(());
    }

    let path = media_path(data_dir, expense.id.unwrap(), file_type);

    let dir = path.parent().unwrap();
    if !dir.exists() {
        std::fs::create_dir(dir)?;
    }

    let mut file = std::fs::File::create(&path)?;

    file.write_all(content)?;

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
