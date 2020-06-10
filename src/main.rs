#![feature(proc_macro_hygiene, decl_macro)]

mod database;
mod error;
mod expense;

use database::Database;
use error::Error;
use rocket_contrib::templates::Template;
use std::collections::HashMap;

type Flash = rocket::response::Flash<rocket::response::Redirect>;
type Result<T> = std::result::Result<T, crate::Error>;

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

#[derive(Clone, Debug)]
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

impl Default for FormData {
    fn default() -> Self {
        Self {
            id: None,
            created_at: String::new(),
            serial: None,
            name: String::new(),
            url: None,
            shop: String::new(),
            warranty: 0,
            price: 0.,
            photo: None,
            invoice: None,
            notice: None,
        }
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

impl Into<expense::Entity> for FormData {
    fn into(self) -> expense::Entity {
        expense::Entity {
            id: self.id,
            created_at: Self::parse_date(&self.created_at),
            serial: self.serial.clone(),
            name: self.name.clone(),
            url: self.url.clone(),
            shop: self.shop.clone(),
            warranty: self.warranty,
            price: self.price,

            warranty_at: Self::parse_date(&self.created_at),
            warranty_active: false,
            trashed: false,
        }
    }
}

fn main() -> Result<()> {
    pretty_env_logger::init();

    #[cfg(debug_assertions)]
    dotenv::dotenv().ok();

    let ip = std::env::var("LISTEN_IP").expect("Missing LISTEN_IP env variable");
    let port = std::env::var("LISTEN_PORT").expect("Missing LISTEN_IP env variable");

    let env = if cfg!(debug_assertions) {
        rocket::config::Environment::Development
    } else {
        rocket::config::Environment::Production
    };

    let mut config = rocket::Config::build(env)
        .address(ip)
        .port(port.parse()?)
        .finalize()?;

    if let Ok(secret_key) = std::env::var("SECRET_KEY") {
        config.set_secret_key(secret_key)?;
    }

    rocket::custom(config)
        .attach(Template::custom(|engines| {
            engines
                .tera
                .register_function("has_media", Box::new(has_media));
        }))
        .manage(Database::new()?)
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
}

#[rocket::get("/?<params..>")]
fn index(database: rocket::State<Database>, params: rocket::request::Form<Params>, flash: Option<rocket::request::FlashMessage>) -> Result<Template> {
    let page = params.page.unwrap_or(1);
    let trashed = params.trashed.unwrap_or(false);
    let limit = params.limit.unwrap_or(50);

    let pager = database.all(page, limit, trashed)?;

    let mut context = tera::Context::new();
    context.insert("pager", &pager);
    context.insert("flash", &flash.map(|x| (x.name().to_string(), x.msg().to_string())));

    Ok(Template::render("expense/list", &context))
}

#[rocket::get("/expenses/add")]
fn add() -> Template {
    let context = tera::Context::new();

    Template::render("expense/edit", &context)
}

#[rocket::post("/expenses/add", data = "<form_data>")]
fn create(database: rocket::State<Database>, form_data: FormData) -> Result<Flash> {
    save(database, -1, form_data)
}

#[rocket::get("/expenses/<id>/edit")]
fn edit(database: rocket::State<Database>, id: i32) -> Result<Option<Template>> {
    Ok(
        database
            .get(id)?
            .map(|expense| Template::render("expense/edit", &expense))
    )
}

#[rocket::post("/expenses/<id>/edit", data = "<form_data>")]
fn save(
    database: rocket::State<Database>,
    id: i32,
    form_data: FormData,
) -> Result<Flash> {
    let entity = form_data.clone().into();

    let (entity, msg) = if id > 0 {
        (database.update(id, &entity)?, "Achat mis à jour")
    } else {
        (database.create(&entity)?, "Achat créé")
    };

    if let Some(photo) = &form_data.photo {
        write_file("photo", photo, &entity)?;
    }
    if let Some(invoice) = &form_data.invoice {
        write_file("invoice", invoice, &entity)?;
    }
    if let Some(notice) = &form_data.notice {
        write_file("notice", notice, &entity)?;
    }

    Ok(Flash::success(rocket::response::Redirect::to("/"), msg))
}

fn write_file(file_type: &str, content: &[u8], expense: &crate::expense::Entity) -> Result<()> {
    use std::io::Write;

    if content.is_empty() {
        return Ok(());
    }

    let path = media_path(expense.id.unwrap(), file_type);

    let dir = path.parent().unwrap();
    if !dir.exists() {
        std::fs::create_dir(dir)?;
    }

    let mut file = std::fs::File::create(&path)?;

    file.write_all(content)?;

    Ok(())
}

#[rocket::get("/expenses/<id>/delete")]
fn delete(database: rocket::State<Database>, id: i32) -> Result<Flash> {
    database.delete(id)?;
    std::fs::remove_dir_all(media_path(id, "photo").parent().unwrap())?;

    Ok(Flash::success(rocket::response::Redirect::to("/"), "Achat supprimé"))
}

#[rocket::get("/expenses/<id>/trash")]
fn trash(database: rocket::State<Database>, id: i32) -> Result<Flash> {
    database.trash(id)?;

    Ok(Flash::success(rocket::response::Redirect::to("/"), "Achat jeté"))
}

#[rocket::get("/expenses/<id>/untrash")]
fn untrash(database: rocket::State<Database>, id: i32) -> Result<Flash> {
    database.untrash(id)?;

    Ok(Flash::success(rocket::response::Redirect::to("/"), "Achat recyclé"))
}

#[rocket::get("/expenses/<id>/photo")]
fn photo(id: i32) -> Option<rocket::response::NamedFile> {
    media(id, "photo")
}

#[rocket::get("/expenses/<id>/invoice")]
fn invoice(id: i32) -> Option<rocket::response::NamedFile> {
    media(id, "invoice")
}

#[rocket::get("/expenses/<id>/notice")]
fn notice(id: i32) -> Option<rocket::response::NamedFile> {
    media(id, "notice")
}

fn media(id: i32, file_type: &str) -> Option<rocket::response::NamedFile> {
    let path = media_path(id, file_type);

    rocket::response::NamedFile::open(&path).ok()
}

fn media_path(id: i32, file_type: &str) -> std::path::PathBuf {
    let data_dir = std::env::var("DATA_DIR").expect("Missing DATA_DIR env variable");

    let filename = format!("{}/{}/{}", data_dir, id, file_type);

    std::path::PathBuf::from(&filename)
}

fn has_media(args: HashMap<String, tera::Value>) -> tera::Result<tera::Value> {
    let id = match args.get("id") {
        Some(val) => tera::from_value::<i32>(val.clone())?,
        None => return Err("oops".into()),
    };

    let file_type = match args.get("file_type") {
        Some(val) => tera::from_value::<String>(val.clone())?,
        None => return Err("oops".into()),
    };

    let exists = media_path(id, &file_type).exists();
    let value = tera::to_value(exists)?;

    Ok(value)
}
