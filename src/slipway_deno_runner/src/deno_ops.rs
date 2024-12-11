use deno_core::*;
use tracing::{debug, error, info, trace, warn};

#[op2(fast)]
fn op_trace(#[string] text: &str) {
    trace!("{}", text);
}

#[op2(fast)]
fn op_debug(#[string] text: &str) {
    debug!("{}", text);
}

#[op2(fast)]
fn op_info(#[string] text: &str) {
    info!("{}", text);
}

#[op2(fast)]
fn op_warn(#[string] text: &str) {
    warn!("{}", text);
}

#[op2(fast)]
fn op_error(#[string] text: &str) {
    error!("{}", text);
}

pub(super) const DECLS: [OpDecl; 5] = [op_trace(), op_debug(), op_info(), op_warn(), op_error()];
