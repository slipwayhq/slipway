use tracing::{debug, error, info, trace, warn};

pub fn log_trace(message: String) {
    trace!(message);
}

pub fn log_debug(message: String) {
    debug!(message);
}

pub fn log_info(message: String) {
    info!(message);
}

pub fn log_warn(message: String) {
    warn!(message);
}

pub fn log_error(message: String) {
    error!(message);
}
