// While we're developing...
#![allow(dead_code)]

use errors::SlipwayError;
use execute::{initialize, AppExecutionState};
use parse::parse_app;

pub use execute::ComponentState;
pub use parse::types::primitives::ComponentHandle;

pub mod errors;
mod execute;
mod parse;
mod utils;

#[cfg(test)]
pub mod test_utils;

pub fn create_app_from_json_string(app_json: &str) -> Result<AppExecutionState, SlipwayError> {
    let app = parse_app(app_json)?;
    initialize(app)
}
