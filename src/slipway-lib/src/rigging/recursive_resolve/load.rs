use async_trait::async_trait;
use thiserror::Error;

use crate::{errors::SlipwayError, rigging::parse::types::UnresolvedComponentReference};

use super::Context;

#[async_trait]
pub(crate) trait LoadComponentRigging {
    async fn load_component_rigging<'a, 'b>(
        &self,
        reference: UnresolvedComponentReference,
        context: &'a Context<'a>,
    ) -> Result<ComponentRigging<'b>, LoadError<'b>>
    where
        'a: 'b;
}

pub(crate) struct ComponentRigging<'a> {
    pub context: &'a Context<'a>,
    pub rigging: String,
}

#[derive(Error, Debug)]
#[error("Load component rigging failed")]
pub(crate) struct LoadError<'a> {
    pub context: &'a Context<'a>,
    pub source: SlipwayError,
}
