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

impl Entity {
    pub fn from(data: &crate::FormData) -> Self {
        Self {
            id: data.id,
            created_at: crate::FormData::parse_date(&data.created_at),
            serial: data.serial.clone(),
            name: data.name.clone(),
            url: data.url.clone(),
            shop: data.shop.clone(),
            warranty: data.warranty,
            price: data.price,
            trashed_at: None,

            warranty_at: crate::FormData::parse_date(&data.created_at),
            warranty_active: false,
            trashed: false,
        }
    }
}

