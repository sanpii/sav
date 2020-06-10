pub struct Database {
    elephantry: elephantry::Pool,
}

impl Database {
    pub fn new() -> elephantry::Result<Self> {
        let database_url =
            std::env::var("DATABASE_URL").expect("Missing DATABASE_URL env variable");

        let elephantry = elephantry::Pool::new(&database_url)?;

        Ok(Self { elephantry })
    }

    pub fn all(
        &self,
        page: usize,
        limit: usize,
        trashed: bool,
    ) -> elephantry::Result<elephantry::Pager<crate::expense::Entity>> {
        let clause = if trashed {
            "trashed_at is not null"
        } else {
            "trashed_at is null"
        };

        self.elephantry
            .paginate_find_where::<crate::expense::Model>(
                clause,
                &[],
                limit,
                page,
                Some("order by created_at desc"),
            )
    }

    pub fn get(&self, id: i32) -> elephantry::Result<Option<crate::expense::Entity>> {
        self.elephantry
            .find_by_pk::<crate::expense::Model>(&elephantry::pk!(id))
    }

    pub fn create(&self, entity: &crate::expense::Entity) -> elephantry::Result<crate::expense::Entity> {
        self.elephantry
            .insert_one::<crate::expense::Model>(entity)
    }

    pub fn update(&self, id: i32, entity: &crate::expense::Entity) -> elephantry::Result<crate::expense::Entity> {
        self.elephantry
            .update_one::<crate::expense::Model>(&elephantry::pk!(id), entity)
    }

    pub fn delete(&self, id: i32) -> elephantry::Result<crate::expense::Entity> {
        self.elephantry
            .delete_by_pk::<crate::expense::Model>(&elephantry::pk!(id))
    }

    pub fn trash(&self, id: i32) -> elephantry::Result<crate::expense::Entity> {
        self.set_trash(id, true)
    }

    pub fn untrash(&self, id: i32) -> elephantry::Result<crate::expense::Entity> {
        self.set_trash(id, false)
    }

    fn set_trash(&self, id: i32, trash: bool) -> elephantry::Result<crate::expense::Entity> {
        let trashed_at = if trash {
            Some(chrono::offset::Local::now().date().naive_local())
        } else {
            None
        };

        let mut data = std::collections::HashMap::new();
        data.insert(
            "trashed_at".to_string(),
            &trashed_at as &dyn elephantry::ToSql,
        );

        self.elephantry
            .update_by_pk::<crate::expense::Model>(&elephantry::pk!(id), &data)
    }
}
