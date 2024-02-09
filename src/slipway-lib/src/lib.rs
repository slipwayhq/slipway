// While we're developing...
#![allow(dead_code)]

use execute::create_session;
pub use execute::{initialize, AppSession, ComponentState};
use parse::parse_app;
pub use parse::types::primitives::ComponentHandle;
pub mod errors;
mod execute;
mod parse;
mod utils;

#[cfg(test)]
pub mod test_utils;

// We export this helper method so we don't have to expose the `App` type.
pub fn create_app_session_from_string(app: &str) -> Result<AppSession, errors::SlipwayError> {
    let app = parse_app(app)?;
    Ok(create_session(app))
}
