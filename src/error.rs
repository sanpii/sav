#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(#[from] std::num::ParseIntError),
    #[error("Sql error: {0}")]
    Sql(#[from] elephantry::Error),
    #[error("Template error: {0}")]
    Template(#[from] tera::Error),
}

impl<'r, 'o: 'r> rocket::response::Responder<'r, 'o> for Error {
    fn respond_to(self, request: &rocket::Request) -> rocket::response::Result<'o> {
        let mut context = tera::Context::new();

        if cfg!(debug_assertions) {
            context.insert("message", &self.to_string());
        }

        let template = rocket_dyn_templates::Template::render("error", &context.into_json());
        let response = rocket::response::content::Html(template);

        response.respond_to(request)
    }
}
