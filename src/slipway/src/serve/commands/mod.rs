mod add_api_key;
mod add_device;
mod add_playlist;
mod add_rig;
mod add_trmnl_device;
mod aot_compile;
mod consolidate;
mod init;

const COMPONENTS_PATH: &str = "components";

pub use add_api_key::add_api_key;
pub use add_device::add_device;
pub use add_playlist::add_playlist;
pub use add_rig::add_rig;
pub use add_trmnl_device::add_trmnl_device;
pub use aot_compile::aot_compile;
pub use consolidate::consolidate;
pub use init::init;
pub use init::init_serve_config;
