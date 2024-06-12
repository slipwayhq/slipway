// While we're developing...
#![allow(dead_code)]

use std::ops::Deref;

pub use execute::app_execution_state::{AppExecutionState, ComponentExecutionData};
pub use execute::app_session::*;
pub use execute::component_state::{
    ComponentInput, ComponentInputOverride, ComponentOutput, ComponentOutputOverride,
    ComponentState,
};
pub use execute::primitives::*;
pub use execute::step::Instruction;
pub use load::basic_components_loader::BasicComponentsLoader;
pub use load::*;
pub use parse::parse_app;
pub use parse::parse_component;
pub use parse::types::primitives::*;
pub use parse::types::slipway_id::*;
pub use parse::types::slipway_reference::*;
pub use parse::types::*;
pub mod errors;
mod execute;
mod load;
mod parse;
pub mod utils;

#[cfg(any(feature = "unstable-test-utils", test))]
mod test_utils;

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
