use std::str::FromStr;

use slipway_engine::{
    BasicComponentCache, CallChain, ComponentHandle, ComponentRigging, Rig, RigSession, Rigging,
    RunComponentError, RunError, SlipwayReference,
};
use slipway_host::run::{no_event_handler, run_rig};

use common::{assert_messages_contains, create_components_loader, get_component_runners};
use serde_json::json;

mod common;

#[test_log::test]
fn test_callout_panic() {
    let rig = create_callout_test_rig(3, "increment", "panic");
    assert_run_errors_with(
        rig,
        &[
            "\"test -> increment -> increment -> increment\"",
            "wasm backtrace",
        ],
    );
}

#[test_log::test]
fn test_callout_error() {
    let rig = create_callout_test_rig(3, "increment", "error");
    assert_run_errors_with(
        rig,
        &[
            "\"test -> increment -> increment -> increment\"",
            "slipway-increment-component-error",
        ],
    );
}

#[test_log::test]
fn test_callout_error_js() {
    let rig = create_callout_test_rig(3, "increment_js", "error");
    assert_run_errors_with(
        rig,
        &[
            "\"test -> increment -> increment -> increment\"",
            "slipway-increment-js-component-error",
        ],
    );
}

#[test_log::test]
fn test_fragment_callout_error() {
    let rig = create_callout_test_rig(3, "fragment", "error");
    assert_run_errors_with(
        rig,
        &[
            "\"test -> first -> increment -> increment -> increment\"",
            "slipway-increment-component-error",
        ],
    );
}

fn create_callout_test_rig(ttl: u32, component_name: &str, result_type: &str) -> Rig {
    Rig::for_test(Rigging {
        components: [(
            ComponentHandle::from_str("test").unwrap(),
            ComponentRigging::for_test_with_reference(
                SlipwayReference::Local {
                    path: format!("slipwayhq.{component_name}.0.0.1.tar").into(),
                },
                Some(json!({
                    "type": "callout_increment",
                    "value": 0,
                    "ttl": ttl,
                    "result_type": result_type
                })),
            ),
        )]
        .into_iter()
        .collect(),
    })
}

#[test_log::test]
fn test_invalid_callout_input() {
    let rig = create_callout_schema_test_rig("increment", "invalid_callout_input");
    assert_run_errors_with(rig, &["\"test -> increment\"", r#""type": "foo""#]);
}

#[test_log::test]
fn test_invalid_callout_output() {
    let rig = create_callout_schema_test_rig("increment", "invalid_callout_output");
    assert_run_errors_with(rig, &["\"test -> increment\"", r#""value": "foo""#]);
}

fn create_callout_schema_test_rig(component_name: &str, call_type: &str) -> Rig {
    Rig::for_test(Rigging {
        components: [(
            ComponentHandle::from_str("test").unwrap(),
            ComponentRigging::for_test_with_reference(
                SlipwayReference::Local {
                    path: format!("slipwayhq.{component_name}.0.0.1.tar").into(),
                },
                Some(json!({
                    "type": call_type,
                })),
            ),
        )]
        .into_iter()
        .collect(),
    })
}

fn assert_run_errors_with(rig: Rig, expected_messages: &[&str]) {
    let component_cache = BasicComponentCache::primed(&rig, &create_components_loader()).unwrap();
    let component_runners = get_component_runners();
    let call_chain = CallChain::full_trust_arc();
    let session = RigSession::new(rig, &component_cache);

    let result = run_rig(
        &session,
        &mut no_event_handler(),
        &component_runners,
        call_chain,
    );

    fn match_inner(error: &anyhow::Error, expected_messages: &[&str]) {
        match error.downcast_ref::<RunError<()>>().unwrap() {
            RunError::RunComponentFailed {
                component_handle: _,
                component_runner: _,
                error,
            } => match error {
                RunComponentError::RunCallReturnedError { message, inner } => {
                    for expected_message in expected_messages {
                        assert_messages_contains(expected_message, message, inner);
                    }
                }
                RunComponentError::RunCallFailed { source } => {
                    match_inner(source, expected_messages);
                }
                _ => panic!(
                    "Expected RunCallReturnedError or RunCallFailed, got: {:#?}",
                    error
                ),
            },
            _ => panic!("Expected RunComponentFailed, got: {:#?}", error),
        }
    }

    match result {
        Ok(_) => panic!("Expected error"),
        Err(error) => {
            match_inner(&error.into(), expected_messages);
        }
    }
}
