#[derive(Debug)]
pub enum Error {
    Config(rocket::config::ConfigError),
    Io(std::io::Error),
    Parse(std::num::ParseIntError),
    Sql(elephantry::Error),
    Template(tera::Error),
}

impl std::error::Error for Error {
}


impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Config(error) => format!("Config error: {}", error),
            Self::Io(error) => format!("I/O error: {}", error),
            Self::Parse(error) => format!("Parse error: {}", error),
            Self::Sql(error) => format!("Sql error: {}", error),
            Self::Template(error) => format!("Template error: {}", error),
        };

        write!(f, "{}", s)
    }
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

impl From<elephantry::Error> for Error {
    fn from(error: elephantry::Error) -> Self {
        Self::Sql(error)
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(error: std::num::ParseIntError) -> Self {
        Self::Parse(error)
    }
}

impl From<rocket::config::ConfigError> for Error {
    fn from(error: rocket::config::ConfigError) -> Self {
        Self::Config(error)
    }
}

impl From<tera::Error> for Error {
    fn from(error: tera::Error) -> Self {
        Self::Template(error)
    }
}
