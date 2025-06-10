// While we're developing...
#![allow(dead_code)]

use std::ops::Deref;
use std::sync::LazyLock;

pub use execute::component_execution_data::permissions::*;
pub use execute::component_execution_data::*;
pub use execute::component_runner::*;
pub use execute::component_state::{
    ComponentInput, ComponentInputOverride, ComponentOutput, ComponentOutputOverride,
    ComponentState,
};
pub use execute::fonts::*;
pub use execute::primitives::*;
pub use execute::rig_execution_state::*;
pub use execute::rig_session::*;
pub use execute::step::*;
pub use load::basic_components_loader::*;
pub use load::special_components::*;
pub use load::*;
pub use parse::types::primitives::*;
pub use parse::types::slipway_id::*;
pub use parse::types::slipway_reference::*;
pub use parse::types::*;
pub use parse::url::*;
pub use parse::*;
use regex::Regex;
pub use special_components::*;
pub mod custom_iter_tools;
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

pub const SLIPWAY_ALPHANUMERIC_NAME_REGEX_STR: &str = r"^[a-z0-9_]+$";
pub static SLIPWAY_ALPHANUMERIC_NAME_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(SLIPWAY_ALPHANUMERIC_NAME_REGEX_STR).expect("Regex should be valid")
});

pub const DEFAULT_FONT_SANS_SERIF: &str = "Roboto";
const ROBOTO_FONT: &[u8] = include_bytes!("../../fonts/Roboto.ttf");

pub const DEFAULT_FONT_MONOSPACE: &str = "Roboto Mono";
const ROBOTO_MONO_FONT: &[u8] = include_bytes!("../../fonts/RobotoMono.ttf");
