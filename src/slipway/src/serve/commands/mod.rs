mod add_device;
mod add_playlist;
mod add_rig;

pub use add_device::add_device;
pub use add_playlist::add_playlist;
pub use add_rig::add_rig;

fn create_friendly_id() -> String {
    nanoid::nanoid!(6)
}

fn create_api_key() -> String {
    nanoid::nanoid!(64)
}
