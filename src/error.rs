#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Config error: {0}")]
    Config(#[from] rocket::config::ConfigError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(#[from] std::num::ParseIntError),
    #[error("Sql error: {0}")]
    Sql(#[from] elephantry::Error),
    #[error("Template error: {0}")]
    Template(#[from] tera::Error),
}

impl<'a> rocket::response::Responder<'a> for Error {
    fn respond_to(self, request: &rocket::Request) -> rocket::response::Result<'a> {
        let mut context = tera::Context::new();

        if cfg!(debug_assertions) {
            context.insert("message", &self.to_string());
        }

        let tera = tera_hot::Template::new(crate::TEMPLATE_DIR);
        let template = tera.render("error.html", &context);
        let response = rocket::response::content::Html(template);

        response.respond_to(request)
    }
}
