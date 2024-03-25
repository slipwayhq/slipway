use async_trait::async_trait;

use crate::{errors::SlipwayError, SlipwayReference};

use super::primitives::LoaderId;

pub(crate) mod local;

#[async_trait]
pub(crate) trait ComponentPartLoader<TResult>: Send + Sync {
    fn id(&self) -> LoaderId;

    async fn load(
        &self,
        component_reference: &SlipwayReference,
    ) -> Result<Option<TResult>, SlipwayError>;
}
