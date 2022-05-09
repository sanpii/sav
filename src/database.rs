#[rocket_sync_db_pools::database("sav")]
pub struct Database(elephantry::Connection);

impl Database {
    pub async fn all(
        &self,
        q: Option<String>,
        page: usize,
        limit: usize,
        trashed: bool,
    ) -> elephantry::Result<elephantry::Pager<crate::expense::Entity>> {
        self.run(move |c| {
            let mut clause = elephantry::Where::new();

            if let Some(q) = &q {
                clause.and_where("name ~* $*", vec![q]);
            }

            if trashed {
                clause.and_where("trashed_at is not null", Vec::new());
            } else {
                clause.and_where("trashed_at is null", Vec::new());
            }

            c.paginate_find_where::<crate::expense::Model>(
                &clause.to_string(),
                &clause.params(),
                limit,
                page,
                Some("order by created_at desc"),
            )
        })
        .await
    }

    pub async fn get(&self, id: i32) -> elephantry::Result<Option<crate::expense::Entity>> {
        self.run(move |c| c.find_by_pk::<crate::expense::Model>(&elephantry::pk!(id)))
            .await
    }

    pub async fn create(
        &self,
        entity: crate::expense::Entity,
    ) -> elephantry::Result<crate::expense::Entity> {
        self.run(move |c| c.insert_one::<crate::expense::Model>(&entity))
            .await
    }

    pub async fn update(
        &self,
        id: i32,
        entity: crate::expense::Entity,
    ) -> elephantry::Result<Option<crate::expense::Entity>> {
        self.run(move |c| c.update_one::<crate::expense::Model>(&elephantry::pk!(id), &entity))
            .await
    }

    pub async fn delete(&self, id: i32) -> elephantry::Result<Option<crate::expense::Entity>> {
        self.run(move |c| c.delete_by_pk::<crate::expense::Model>(&elephantry::pk!(id)))
            .await
    }

    pub async fn trash(&self, id: i32) -> elephantry::Result<Option<crate::expense::Entity>> {
        self.set_trash(id, true).await
    }

    pub async fn untrash(&self, id: i32) -> elephantry::Result<Option<crate::expense::Entity>> {
        self.set_trash(id, false).await
    }

    async fn set_trash(
        &self,
        id: i32,
        trash: bool,
    ) -> elephantry::Result<Option<crate::expense::Entity>> {
        self.run(move |c| {
            let trashed_at = if trash {
                Some(chrono::offset::Local::now().date_naive())
            } else {
                None
            };

            let mut data = std::collections::HashMap::new();
            data.insert(
                "trashed_at".to_string(),
                &trashed_at as &dyn elephantry::ToSql,
            );

            c.update_by_pk::<crate::expense::Model>(&elephantry::pk!(id), &data)
        })
        .await
    }

    pub async fn shops(&self) -> elephantry::Result<Vec<String>> {
        self.run(move |c| {
            let rows = c.query("select distinct shop from expense order by 1", &[])?;

            Ok(rows.into_vec())
        })
        .await
    }
}
