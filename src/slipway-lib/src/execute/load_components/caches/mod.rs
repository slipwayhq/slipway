use crate::{Component, SlipwayReference};

use super::try_load_component_part::LoadComponentResult;

pub(crate) mod in_memory;

pub(crate) trait LoadedComponentCache {
    fn prime_cache_for(&mut self, component_reference: &SlipwayReference);

    fn get_definition(
        &mut self,
        component_reference: &SlipwayReference,
    ) -> &LoadComponentResult<Component<jtd::Schema>>;

    fn get_wasm(&mut self, component_reference: &SlipwayReference)
        -> &LoadComponentResult<Vec<u8>>;
}
