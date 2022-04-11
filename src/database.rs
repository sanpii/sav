#[rocket_contrib::database("sav")]
pub struct Database(elephantry::Connection);

impl Database {
    pub fn all(
        &self,
        q: &Option<String>,
        page: usize,
        limit: usize,
        trashed: bool,
    ) -> elephantry::Result<elephantry::Pager<crate::expense::Entity>> {
        let mut clause = elephantry::Where::new();

        if let Some(q) = q {
            clause.and_where("name ~* $*", vec![q]);
        }

        if trashed {
            clause.and_where("trashed_at is not null", Vec::new());
        } else {
            clause.and_where("trashed_at is null", Vec::new());
        }

        self.0.paginate_find_where::<crate::expense::Model>(
            &clause.to_string(),
            &clause.params(),
            limit,
            page,
            Some("order by created_at desc"),
        )
    }

    pub fn get(&self, id: i32) -> elephantry::Result<Option<crate::expense::Entity>> {
        self.0
            .find_by_pk::<crate::expense::Model>(&elephantry::pk!(id))
    }

    pub fn create(
        &self,
        entity: &crate::expense::Entity,
    ) -> elephantry::Result<crate::expense::Entity> {
        self.0.insert_one::<crate::expense::Model>(entity)
    }

    pub fn update(
        &self,
        id: i32,
        entity: &crate::expense::Entity,
    ) -> elephantry::Result<Option<crate::expense::Entity>> {
        self.0
            .update_one::<crate::expense::Model>(&elephantry::pk!(id), entity)
    }

    pub fn delete(&self, id: i32) -> elephantry::Result<Option<crate::expense::Entity>> {
        self.0
            .delete_by_pk::<crate::expense::Model>(&elephantry::pk!(id))
    }

    pub fn trash(&self, id: i32) -> elephantry::Result<Option<crate::expense::Entity>> {
        self.set_trash(id, true)
    }

    pub fn untrash(&self, id: i32) -> elephantry::Result<Option<crate::expense::Entity>> {
        self.set_trash(id, false)
    }

    fn set_trash(
        &self,
        id: i32,
        trash: bool,
    ) -> elephantry::Result<Option<crate::expense::Entity>> {
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

        self.0
            .update_by_pk::<crate::expense::Model>(&elephantry::pk!(id), &data)
    }

    pub fn shops(&self) -> elephantry::Result<Vec<String>> {
        let rows = self.0.query("select distinct shop from expense order by 1", &[])?;

        Ok(rows.into_vec())
    }
}
