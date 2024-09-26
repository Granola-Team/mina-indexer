use super::db;
use crate::{constants, store::version::VersionStore};
use async_graphql::{Context, Object};

#[derive(Default)]
pub struct VersionQueryRoot;

#[Object]
impl VersionQueryRoot {
    async fn version(&self) -> String {
        constants::VERSION.to_string()
    }

    async fn db_version(&self, ctx: &Context<'_>) -> anyhow::Result<String> {
        let db = db(ctx);
        db.get_db_version().map(|v| v.to_string())
    }
}
