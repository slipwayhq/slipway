use std::sync::Arc;

use crate::{ComponentError, permissions::log_permissions_check};
use slipway_engine::{CallChain, ComponentExecutionContext, Permission};

pub fn ensure_can_fetch_env(
    key: &str,
    execution_context: &ComponentExecutionContext,
) -> Result<(), ComponentError> {
    log_permissions_check(&format!("access environment variable: {key}"));
    ensure_can_fetch_env_inner(key, Arc::clone(&execution_context.call_chain))
}

fn ensure_can_fetch_env_inner(
    key: &str,
    call_chain: Arc<CallChain<'_>>,
) -> Result<(), ComponentError> {
    let is_allowed = slipway_engine::ensure_permissions(Arc::clone(&call_chain), |permissions| {
        fn matches(key: &str, permission: &Permission) -> bool {
            match permission {
                Permission::All => true,
                Permission::Env(permission) => permission.matches(key),
                _ => false,
            }
        }

        for permission in permissions.deny {
            if matches(key, permission) {
                super::warn_deny_permission_triggered(permission);
                return false;
            }
        }

        for permission in permissions.allow {
            if matches(key, permission) {
                return true;
            }
        }

        false
    });

    if !is_allowed {
        let message = format!(
            "{} does not have permission to fetch environment variable \"{}\"",
            call_chain.rig_or_component_handle_trail_error_prefix(),
            key
        );
        return Err(super::create_permission_error(message, &call_chain));
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use slipway_engine::StringPermission;
    use slipway_engine::UrlPermission;
    use slipway_engine::{ComponentHandle, Permissions, utils::ch};

    use super::*;

    static CH: std::sync::OnceLock<ComponentHandle> = std::sync::OnceLock::new();

    fn run_test(key: &str, permissions: Permissions, expected: bool) {
        let handle = CH.get_or_init(|| ch("test"));
        let call_chain = Arc::new(CallChain::new_for_component(handle, permissions));
        assert_eq!(
            ensure_can_fetch_env_inner(key, call_chain.clone()).is_ok(),
            expected
        );
    }

    mod insufficient_permissions {
        use super::*;

        #[test]
        fn it_should_forbid_any_query_when_no_permissions() {
            run_test("FOO_BAR", Permissions::empty(), false);
        }

        #[test]
        fn it_should_forbid_any_query_with_incorrect_permissions() {
            run_test(
                "FOO_BAR",
                Permissions::allow(&vec![Permission::Http(UrlPermission::Any {})]),
                false,
            );
        }
    }

    mod allow_all {
        use super::*;

        #[test]
        fn it_should_allow_any_query() {
            run_test("FOO_BAR", Permissions::allow(&vec![Permission::All]), true);
        }
    }

    mod query_any {
        use super::*;

        #[test]
        fn it_should_allow_any_query() {
            run_test(
                "FOO_BAR",
                Permissions::allow(&vec![Permission::Env(StringPermission::Any {})]),
                true,
            );
        }
    }

    mod query_exact {
        use super::*;

        fn create_permissions() -> Vec<Permission> {
            vec![Permission::Env(StringPermission::Exact {
                exact: "FOO_BAR".to_string(),
            })]
        }

        #[test]
        fn it_should_allow_exact_query() {
            let permissions = create_permissions();

            run_test("FOO_BAR", Permissions::allow(&permissions), true);
            run_test("FOO_BAR_BAZ", Permissions::allow(&permissions), false);
            run_test("FOO", Permissions::allow(&permissions), false);
        }
    }

    mod query_prefix {
        use super::*;

        fn create_permissions() -> Vec<Permission> {
            vec![Permission::Env(StringPermission::Prefix {
                prefix: "FOO_BAR".to_string(),
            })]
        }

        #[test]
        fn it_should_allow_query_with_prefix() {
            let permissions = create_permissions();

            run_test("FOO_BAR", Permissions::allow(&permissions), true);
            run_test("FOO_BAR_BAZ", Permissions::allow(&permissions), true);
            run_test("FOO", Permissions::allow(&permissions), false);
            run_test("BAR_FOO", Permissions::allow(&permissions), false);
        }
    }

    mod query_suffix {
        use super::*;

        fn create_permissions() -> Vec<Permission> {
            vec![Permission::Env(StringPermission::Suffix {
                suffix: "FOO_BAR".to_string(),
            })]
        }

        #[test]
        fn it_should_allow_query_with_prefix() {
            let permissions = create_permissions();

            run_test("FOO_BAR", Permissions::allow(&permissions), true);
            run_test("FOO_BAR_BAZ", Permissions::allow(&permissions), false);
            run_test("FOO", Permissions::allow(&permissions), false);
            run_test("BAZ_FOO_BAR", Permissions::allow(&permissions), true);
        }
    }

    mod query_deny_exact {
        use super::*;

        fn create_allow_permissions() -> Vec<Permission> {
            vec![Permission::Env(StringPermission::Prefix {
                prefix: "FOO_BAR".to_string(),
            })]
        }

        fn create_deny_permissions() -> Vec<Permission> {
            vec![Permission::Env(StringPermission::Exact {
                exact: "FOO_BAR_BAZ".to_string(),
            })]
        }

        #[test]
        fn it_should_deny_exact_font() {
            let allow_permissions = create_allow_permissions();
            let deny_permissions = create_deny_permissions();

            run_test(
                "FOO_BAR",
                Permissions::new(&allow_permissions, &deny_permissions),
                true,
            );
            run_test(
                "FOO_BAR_QUX",
                Permissions::new(&allow_permissions, &deny_permissions),
                true,
            );
            run_test(
                "FOO_BAR_BAZ",
                Permissions::new(&allow_permissions, &deny_permissions),
                false,
            );
            run_test(
                "FOO_BAR_BAZ_QUX",
                Permissions::new(&allow_permissions, &deny_permissions),
                true,
            );
        }
    }

    mod query_deny_prefix {
        use super::*;

        fn create_allow_permissions() -> Vec<Permission> {
            vec![Permission::Env(StringPermission::Prefix {
                prefix: "FOO_BAR".to_string(),
            })]
        }

        fn create_deny_permissions() -> Vec<Permission> {
            vec![Permission::Env(StringPermission::Prefix {
                prefix: "FOO_BAR_BAZ".to_string(),
            })]
        }

        #[test]
        fn it_should_deny_font_prefix() {
            let allow_permissions = create_allow_permissions();
            let deny_permissions = create_deny_permissions();

            run_test(
                "FOO_BAR",
                Permissions::new(&allow_permissions, &deny_permissions),
                true,
            );
            run_test(
                "FOO_BAR_QUX",
                Permissions::new(&allow_permissions, &deny_permissions),
                true,
            );
            run_test(
                "FOO_BAR_BAZ",
                Permissions::new(&allow_permissions, &deny_permissions),
                false,
            );
            run_test(
                "FOO_BAR_BAZ_QUX",
                Permissions::new(&allow_permissions, &deny_permissions),
                false,
            );
        }
    }
}
