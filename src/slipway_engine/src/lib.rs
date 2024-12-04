// While we're developing...
#![allow(dead_code)]

use std::ops::Deref;

pub use execute::component_execution_data::*;
pub use execute::component_runner::*;
pub use execute::component_state::{
    ComponentInput, ComponentInputOverride, ComponentOutput, ComponentOutputOverride,
    ComponentState,
};
pub use execute::primitives::*;
pub use execute::rig_execution_state::*;
pub use execute::rig_session::*;
pub use execute::step::*;
pub use load::basic_components_loader::BasicComponentsLoader;
pub use load::*;
pub use parse::parse_component;
pub use parse::parse_rig;
pub use parse::types::primitives::*;
pub use parse::types::slipway_id::*;
pub use parse::types::slipway_reference::*;
pub use parse::types::*;
pub use special_components::SpecialComponentRunner;
pub mod errors;
mod execute;
mod load;
mod parse;
mod special_components;
pub mod utils;

#[cfg(any(feature = "unstable-test-utils", test))]
pub mod test_utils;

pub struct Immutable<T> {
    value: T,
}

impl<T> Immutable<T> {
    pub fn new(value: T) -> Self {
        Immutable { value }
    }
}

impl<T> Deref for Immutable<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
