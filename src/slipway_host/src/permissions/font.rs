use std::sync::Arc;

use slipway_engine::{CallChain, ComponentExecutionContext, Permission};

use crate::ComponentError;

pub fn ensure_can_query_font(
    query: &str,
    execution_context: &ComponentExecutionContext,
) -> Result<(), ComponentError> {
    ensure_can_query_font_inner(query, Arc::clone(&execution_context.call_chain))
}

fn ensure_can_query_font_inner(
    query: &str,
    call_chain: Arc<CallChain<'_>>,
) -> Result<(), ComponentError> {
    let is_allowed = slipway_engine::ensure_permissions(Arc::clone(&call_chain), |permissions| {
        fn matches(query: &str, permission: &Permission) -> bool {
            match permission {
                Permission::All => true,
                Permission::FontQuery(permission) => permission.matches(query),
                _ => false,
            }
        }

        for permission in permissions.deny {
            if matches(query, permission) {
                super::warn_deny_permission_triggered(permission);
                return false;
            }
        }

        for permission in permissions.allow {
            if matches(query, permission) {
                return true;
            }
        }

        false
    });

    if !is_allowed {
        let message = format!(
            "Component {} does not have permission to perform font query {}",
            call_chain.component_handle_trail(),
            query
        );
        return Err(super::create_permission_error(message, &call_chain));
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use slipway_engine::StringPermission;
    use slipway_engine::UrlPermission;
    use slipway_engine::{utils::ch, ComponentHandle, Permissions};

    use super::*;

    static CH: std::sync::OnceLock<ComponentHandle> = std::sync::OnceLock::new();

    fn run_test(query: &str, permissions: Permissions, expected: bool) {
        let handle = CH.get_or_init(|| ch("test"));
        let call_chain = Arc::new(CallChain::new_for_component(handle, permissions));
        assert_eq!(
            ensure_can_query_font_inner(query, call_chain.clone()).is_ok(),
            expected
        );
    }

    mod insufficient_permissions {
        use super::*;

        #[test]
        fn it_should_forbid_any_query_when_no_permissions() {
            run_test("Roboto", Permissions::empty(), false);
        }

        #[test]
        fn it_should_forbid_any_query_with_incorrect_permissions() {
            run_test(
                "Roboto",
                Permissions::allow(&vec![Permission::HttpFetch(UrlPermission::Any)]),
                false,
            );
        }
    }

    mod allow_all {
        use super::*;

        #[test]
        fn it_should_allow_any_query() {
            run_test("Roboto", Permissions::allow(&vec![Permission::All]), true);
        }
    }

    mod query_any {
        use super::*;

        #[test]
        fn it_should_allow_any_query() {
            run_test(
                "Roboto",
                Permissions::allow(&vec![Permission::FontQuery(StringPermission::Any)]),
                true,
            );
        }
    }

    mod query_exact {
        use super::*;

        fn create_permissions() -> Vec<Permission> {
            vec![Permission::FontQuery(StringPermission::Exact(
                "Roboto".to_string(),
            ))]
        }

        #[test]
        fn it_should_allow_exact_query() {
            let permissions = create_permissions();

            run_test("Roboto", Permissions::allow(&permissions), true);
            run_test("Roboto Mono", Permissions::allow(&permissions), false);
            run_test("Robot", Permissions::allow(&permissions), false);
        }
    }

    mod query_prefix {
        use super::*;

        fn create_permissions() -> Vec<Permission> {
            vec![Permission::FontQuery(StringPermission::Prefix(
                "Roboto".to_string(),
            ))]
        }

        #[test]
        fn it_should_allow_query_with_prefix() {
            let permissions = create_permissions();

            run_test("Roboto", Permissions::allow(&permissions), true);
            run_test("Roboto Mono", Permissions::allow(&permissions), true);
            run_test("Robot", Permissions::allow(&permissions), false);
            run_test("Mono Roboto", Permissions::allow(&permissions), false);
        }
    }

    mod query_suffix {
        use super::*;

        fn create_permissions() -> Vec<Permission> {
            vec![Permission::FontQuery(StringPermission::Suffix(
                "Roboto".to_string(),
            ))]
        }

        #[test]
        fn it_should_allow_query_with_prefix() {
            let permissions = create_permissions();

            run_test("Roboto", Permissions::allow(&permissions), true);
            run_test("Roboto Mono", Permissions::allow(&permissions), false);
            run_test("Robot", Permissions::allow(&permissions), false);
            run_test("Mono Roboto", Permissions::allow(&permissions), true);
        }
    }

    mod query_deny_exact {
        use super::*;

        fn create_allow_permissions() -> Vec<Permission> {
            vec![Permission::FontQuery(StringPermission::Prefix(
                "Roboto".to_string(),
            ))]
        }

        fn create_deny_permissions() -> Vec<Permission> {
            vec![Permission::FontQuery(StringPermission::Exact(
                "Roboto Mono".to_string(),
            ))]
        }

        #[test]
        fn it_should_deny_exact_font() {
            let allow_permissions = create_allow_permissions();
            let deny_permissions = create_deny_permissions();

            run_test(
                "Roboto",
                Permissions::new(&allow_permissions, &deny_permissions),
                true,
            );
            run_test(
                "Roboto Sans",
                Permissions::new(&allow_permissions, &deny_permissions),
                true,
            );
            run_test(
                "Roboto Mono",
                Permissions::new(&allow_permissions, &deny_permissions),
                false,
            );
            run_test(
                "Roboto Monospace",
                Permissions::new(&allow_permissions, &deny_permissions),
                true,
            );
        }
    }

    mod query_deny_prefix {
        use super::*;

        fn create_allow_permissions() -> Vec<Permission> {
            vec![Permission::FontQuery(StringPermission::Prefix(
                "Roboto".to_string(),
            ))]
        }

        fn create_deny_permissions() -> Vec<Permission> {
            vec![Permission::FontQuery(StringPermission::Prefix(
                "Roboto Mono".to_string(),
            ))]
        }

        #[test]
        fn it_should_deny_font_prefix() {
            let allow_permissions = create_allow_permissions();
            let deny_permissions = create_deny_permissions();

            run_test(
                "Roboto",
                Permissions::new(&allow_permissions, &deny_permissions),
                true,
            );
            run_test(
                "Roboto Sans",
                Permissions::new(&allow_permissions, &deny_permissions),
                true,
            );
            run_test(
                "Roboto Mono",
                Permissions::new(&allow_permissions, &deny_permissions),
                false,
            );
            run_test(
                "Roboto Monospace",
                Permissions::new(&allow_permissions, &deny_permissions),
                false,
            );
        }
    }
}
