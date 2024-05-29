use crate::constants;
use async_graphql::Object;

#[derive(Default)]
pub struct VersionQueryRoot;

#[Object]
impl VersionQueryRoot {
    async fn version(&self) -> String {
        constants::VERSION.to_string()
    }
}
