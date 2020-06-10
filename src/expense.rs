#[derive(Debug, elephantry::Entity, serde_derive::Serialize)]
pub struct Entity {
    pub id: Option<i32>,
    pub created_at: chrono::NaiveDateTime,
    pub serial: Option<String>,
    pub name: String,
    pub url: Option<String>,
    pub shop: String,
    pub warranty: i32,
    pub price: f32,
    pub warranty_at: chrono::NaiveDateTime,
    pub warranty_active: bool,
    pub trashed: bool,
}

pub struct Model;

impl<'a> elephantry::Model<'a> for Model {
    type Entity = Entity;
    type Structure = Structure;

    fn new(_: &'a elephantry::Connection) -> Self {
        Self {}
    }

    fn create_projection() -> elephantry::Projection {
        Self::default_projection()
            .add_field(
                "warranty_at",
                "%:created_at:% + (%:warranty:% || ' years')::interval",
            )
            .add_field(
                "warranty_active",
                "%:created_at:% + (%:warranty:% || ' years')::interval > now()",
            )
            .add_field("trashed", "%:trashed_at:% is not null")
            .unset_field("trashed_at")
    }
}

pub struct Structure;

impl elephantry::Structure for Structure {
    fn relation() -> &'static str {
        "public.expense"
    }

    fn primary_key() -> &'static [&'static str] {
        &["id"]
    }

    fn definition() -> &'static [&'static str] {
        &[
            "id",
            "created_at",
            "serial",
            "name",
            "url",
            "shop",
            "warranty",
            "price",
            "trashed_at",
        ]
    }
}
