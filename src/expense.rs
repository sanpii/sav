#[derive(Debug, elephantry::Entity, rocket::serde::Serialize)]
#[elephantry(model = "Model", structure = "Structure", relation = "public.expense")]
#[serde(crate = "rocket::serde")]
pub struct Entity {
    #[elephantry(pk)]
    pub id: Option<i32>,
    pub created_at: chrono::NaiveDateTime,
    pub serial: Option<String>,
    pub name: String,
    pub url: Option<String>,
    pub shop: String,
    pub warranty: i32,
    pub price: f32,
    pub trashed_at: Option<chrono::NaiveDateTime>,

    #[elephantry(virtual = "%:created_at:% + (%:warranty:% || ' years')::interval")]
    pub warranty_at: chrono::NaiveDateTime,
    #[elephantry(virtual = "%:created_at:% + (%:warranty:% || ' years')::interval > now()")]
    pub warranty_active: bool,
    #[elephantry(virtual = "%:trashed_at:% is not null")]
    pub trashed: bool,
}
