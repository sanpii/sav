#![feature(proc_macro_hygiene, decl_macro)]
#![warn(warnings)]

mod database;
mod error;
mod expense;

use database::Database;
use error::Error;
use std::collections::HashMap;

type Flash = rocket::response::Flash<rocket::response::Redirect>;
type Result<T> = std::result::Result<T, crate::Error>;
type Response = rocket::response::content::Html<String>;

static TEMPLATE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/templates");

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
            .and_hms(0, 0, 0)
    }
}

impl<'a> rocket::data::FromData<'a> for FormData {
    type Owned = Vec<u8>;
    type Borrowed = [u8];
    type Error = crate::Error;

    fn transform(
        _request: &rocket::Request,
        data: rocket::Data,
    ) -> rocket::data::Transform<rocket::data::Outcome<Self::Owned, Self::Error>> {
        let mut d = Vec::new();
        data.stream_to(&mut d).expect("Unable to read");

        rocket::data::Transform::Owned(rocket::data::Outcome::Success(d))
    }

    fn from_data(
        request: &rocket::Request,
        outcome: rocket::data::Transformed<'a, Self>,
    ) -> rocket::data::Outcome<Self, Self::Error> {
        let d = outcome.owned()?;

        let ct = request
            .headers()
            .get_one("Content-Type")
            .expect("no content-type");
        let idx = ct.find("boundary=").expect("no boundary");
        let boundary = &ct[(idx + "boundary=".len())..];

        let mut mp = multipart::server::Multipart::with_body(&d[..], boundary);

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

            warranty_at: FormData::parse_date(&data.created_at),
            warranty_active: false,
            trashed: false,
        }
    }
}

struct AppData {
    pub template: tera_hot::Template,
}

impl AppData {
    fn new() -> Result<Self> {
        let mut template = tera_hot::Template::new(TEMPLATE_DIR);
        template.register_function("has_media", Box::new(has_media));
        template.register_function("pager", elephantry_extras::tera::Pager);

        let app_data = Self { template };

        Ok(app_data)
    }
}

fn main() -> Result<()> {
    rocket::ignite()
        .attach(Database::fairing())
        .attach(rocket::fairing::AdHoc::on_attach(
            "data_dir config",
            |rocket| {
                let data_dir = rocket
                    .config()
                    .get_str("data_dir")
                    .unwrap_or("data/")
                    .to_string();

                Ok(rocket.manage(DataDir(data_dir)))
            },
        ))
        .manage(AppData::new()?)
        .mount(
            "/static",
            rocket_contrib::serve::StaticFiles::from(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/static"
            )),
        )
        .mount(
            "/",
            rocket::routes![
                index, add, create, edit, save, delete, trash, untrash, photo, invoice, notice,
            ],
        )
        .launch();

    Ok(())
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
fn index(
    database: Database,
    data_dir: rocket::State<DataDir>,
    data: rocket::State<AppData>,
    params: rocket::request::Form<Params>,
    flash: Option<rocket::request::FlashMessage>,
) -> Result<Response> {
    let page = params.page.unwrap_or(1);
    let trashed = params.trashed.unwrap_or(false);
    let limit = params.limit.unwrap_or(50);

    let pager = database.all(&params.q, page, limit, trashed)?;

    let base_url = if let Some(q) = &params.q {
        format!("/?q={}", q)
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
        &flash.map(|x| (x.name().to_string(), x.msg().to_string())),
    );

    let template = data.template.render("expense/list.html", &context)?;

    Ok(rocket::response::content::Html(template))
}

#[rocket::get("/expenses/add")]
fn add(data: rocket::State<AppData>) -> Result<Response> {
    let context = tera::Context::new();
    let template = data.template.render("expense/edit.html", &context)?;

    Ok(rocket::response::content::Html(template))
}

#[rocket::post("/expenses/add", data = "<form_data>")]
fn create(
    database: Database,
    data_dir: rocket::State<DataDir>,
    form_data: FormData,
) -> Result<Flash> {
    save(database, data_dir, -1, form_data)
}

#[rocket::get("/expenses/<id>/edit")]
fn edit(database: Database, data: rocket::State<AppData>, id: i32) -> Result<Option<Response>> {
    let expense = match database.get(id)? {
        Some(expense) => expense,
        None => return Ok(None),
    };
    let context = tera::Context::from_serialize(expense)?;
    let template = data.template.render("expense/edit.html", &context)?;
    let response = rocket::response::content::Html(template);

    Ok(Some(response))
}

#[rocket::post("/expenses/<id>/edit", data = "<form_data>")]
fn save(
    database: Database,
    data_dir: rocket::State<DataDir>,
    id: i32,
    form_data: FormData,
) -> Result<Flash> {
    let entity = form_data.clone().into();

    let (entity, msg) = if id > 0 {
        (database.update(id, &entity)?.unwrap(), "Achat mis à jour")
    } else {
        (database.create(&entity)?, "Achat créé")
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
) -> Result<()> {
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
fn delete(database: Database, data_dir: rocket::State<DataDir>, id: i32) -> Result<Flash> {
    database.delete(id)?;
    std::fs::remove_dir_all(media_path(&data_dir.0, id, "photo").parent().unwrap())?;

    Ok(Flash::success(
        rocket::response::Redirect::to("/"),
        "Achat supprimé",
    ))
}

#[rocket::get("/expenses/<id>/trash")]
fn trash(database: Database, id: i32) -> Result<Flash> {
    database.trash(id)?;

    Ok(Flash::success(
        rocket::response::Redirect::to("/"),
        "Achat jeté",
    ))
}

#[rocket::get("/expenses/<id>/untrash")]
fn untrash(database: Database, id: i32) -> Result<Flash> {
    database.untrash(id)?;

    Ok(Flash::success(
        rocket::response::Redirect::to("/"),
        "Achat recyclé",
    ))
}

#[rocket::get("/expenses/<id>/photo")]
fn photo(data_dir: rocket::State<DataDir>, id: i32) -> Option<rocket::response::NamedFile> {
    media(&data_dir.0, id, "photo")
}

#[rocket::get("/expenses/<id>/invoice")]
fn invoice(data_dir: rocket::State<DataDir>, id: i32) -> Option<rocket::response::NamedFile> {
    media(&data_dir.0, id, "invoice")
}

#[rocket::get("/expenses/<id>/notice")]
fn notice(data_dir: rocket::State<DataDir>, id: i32) -> Option<rocket::response::NamedFile> {
    media(&data_dir.0, id, "notice")
}

fn media(data_dir: &str, id: i32, file_type: &str) -> Option<rocket::response::NamedFile> {
    let path = media_path(data_dir, id, file_type);

    rocket::response::NamedFile::open(&path).ok()
}

fn media_path(data_dir: &str, id: i32, file_type: &str) -> std::path::PathBuf {
    let filename = format!("{}/{}/{}", data_dir, id, file_type);

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
