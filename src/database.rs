pub struct Database {
    elephantry: elephantry::Pool,
}

impl Database
{
    pub fn new() -> Self
    {
        let database_url = std::env::var("DATABASE_URL")
            .expect("Missing DATABASE_URL env variable");

        let elephantry = elephantry::Pool::new(&database_url)
            .unwrap();

        Self {
            elephantry,
        }
    }

    fn connection(&self) -> &elephantry::Connection
    {
        self.elephantry.get_default().unwrap()
    }

    pub fn all(&self, page: usize, limit: usize, trashed: bool) -> elephantry::Pager<crate::expense::Entity>
    {
        let clause = if trashed {
            "trashed_at is not null"
        }
        else {
            "trashed_at is null"
        };

        self.connection().paginate_find_where::<crate::expense::Model>(clause, &[], limit, page, Some("order by created_at desc"))
            .unwrap()
    }

    pub fn get(&self, id: i32) -> Option<crate::expense::Entity>
    {
        self.connection().find_by_pk::<crate::expense::Model>(&elephantry::pk!(id))
            .unwrap()
    }

    pub fn create(&self, entity: &crate::expense::Entity) -> crate::expense::Entity
    {
        self.connection()
            .insert_one::<crate::expense::Model>(entity)
            .unwrap()
    }

    pub fn update(&self, id: i32, entity: &crate::expense::Entity) -> crate::expense::Entity
    {
        self.connection()
            .update_one::<crate::expense::Model>(&elephantry::pk!(id), entity)
            .unwrap()
    }

    pub fn delete(&self, id: i32) -> crate::expense::Entity
    {
        self.connection()
            .delete_by_pk::<crate::expense::Model>(&elephantry::pk!(id))
            .unwrap()
    }

    pub fn trash(&self, id: i32)
    {
        self.set_trash(id, true)
    }

    pub fn untrash(&self, id: i32)
    {
        self.set_trash(id, false)
    }

    fn set_trash(&self, id: i32, trash: bool)
    {
        let trashed_at = if trash {
            Some(chrono::offset::Local::now().date().naive_local())
        } else {
            None
        };

        let mut data = std::collections::HashMap::new();
        data.insert("trashed_at".to_string(), &trashed_at as &dyn elephantry::ToSql);

        self.connection()
            .update_by_pk::<crate::expense::Model>(
                &elephantry::pk!(id),
                &data,
            )
            .unwrap();
    }
}
