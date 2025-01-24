use std::sync::Arc;

use crate::ComponentError;
use semver::Version;
use slipway_engine::{
    CallChain, ComponentExecutionContext, ComponentHandle, Permission, SlipwayReference,
};
use url::Url;

pub fn ensure_can_use_component_handle(
    handle: &ComponentHandle,
    execution_context: &ComponentExecutionContext,
) -> Result<(), ComponentError> {
    let component_reference = execution_context
        .callout_context
        .get_component_reference_for_handle(handle)
        .map_err(|e| ComponentError {
            message: format!(
                "Failed to component reference for \"{}\"",
                execution_context
                    .call_chain
                    .component_handle_trail_for(handle)
            ),
            inner: vec![format!("{e}")],
        })?;

    let call_chain = Arc::clone(&execution_context.call_chain);

    ensure_can_use_component_reference(component_reference, call_chain)
}

pub fn ensure_can_use_component_reference(
    component_reference: &SlipwayReference,
    call_chain: Arc<CallChain<'_>>,
) -> Result<(), ComponentError> {
    let is_allowed = match component_reference {
        SlipwayReference::Registry {
            publisher,
            name,
            version,
        } => {
            fn matches(
                publisher: &str,
                name: &str,
                version: &Version,
                permission: &Permission,
            ) -> bool {
                match permission {
                    Permission::All => true,
                    Permission::RegistryComponent(permission) => {
                        permission.matches(publisher, name, version)
                    }
                    _ => false,
                }
            }

            slipway_engine::ensure_permissions(call_chain.clone(), |permissions| {
                for permission in permissions.deny {
                    if matches(publisher, name, version, permission) {
                        super::warn_deny_permission_triggered(permission);
                        return false;
                    }
                }

                for permission in permissions.allow {
                    if matches(publisher, name, version, permission) {
                        return true;
                    }
                }

                false
            })
        }
        SlipwayReference::Http { url } => {
            fn matches(url: &Url, permission: &Permission) -> bool {
                match permission {
                    Permission::All => true,
                    Permission::HttpComponent(permission) => permission.matches(url),
                    _ => false,
                }
            }

            slipway_engine::ensure_permissions(call_chain.clone(), |permissions| {
                for permission in permissions.deny {
                    if matches(url, permission) {
                        super::warn_deny_permission_triggered(permission);
                        return false;
                    }
                }

                for permission in permissions.allow {
                    if matches(url, permission) {
                        return true;
                    }
                }

                false
            })
        }
        SlipwayReference::Local { path: _ } => {
            fn matches(path: &str, permission: &Permission) -> bool {
                match permission {
                    Permission::All => true,
                    Permission::LocalComponent(permission) => permission.matches(path),
                    _ => false,
                }
            }

            let local_reference_string = component_reference.to_string();

            slipway_engine::ensure_permissions(call_chain.clone(), |permissions| {
                for permission in permissions.deny {
                    if matches(&local_reference_string, permission) {
                        super::warn_deny_permission_triggered(permission);
                        return false;
                    }
                }

                for permission in permissions.allow {
                    if matches(&local_reference_string, permission) {
                        return true;
                    }
                }

                false
            })
        }
        SlipwayReference::Special(_) => true,
    };

    if !is_allowed {
        let message = format!(
            "{} does not have permission to access component \"{}\"",
            call_chain.rig_or_component_handle_trail_error_prefix(),
            component_reference
        );
        return Err(super::create_permission_error(message, &call_chain));
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use semver::VersionReq;
    use std::str::FromStr;

    use slipway_engine::RegistryComponentPermission;
    use slipway_engine::UrlPermission;
    use slipway_engine::{utils::ch, ComponentHandle, Permissions};

    use super::*;

    static CH: std::sync::OnceLock<ComponentHandle> = std::sync::OnceLock::new();

    fn run_test(reference_str: &str, permissions: Permissions, expected: bool) {
        let component_reference = SlipwayReference::from_str(reference_str).unwrap();
        let handle = CH.get_or_init(|| ch("test"));
        let call_chain = Arc::new(CallChain::new_for_component(handle, permissions));
        assert_eq!(
            ensure_can_use_component_reference(&component_reference, call_chain.clone()).is_ok(),
            expected
        );
    }

    mod insufficient_permissions {
        use super::*;

        #[test]
        fn it_should_forbid_any_query_when_no_permissions() {
            run_test("p1.n1.1.0.1", Permissions::empty(), false);
        }

        #[test]
        fn it_should_forbid_any_query_with_incorrect_permissions() {
            run_test(
                "p1.n1.1.0.1",
                Permissions::allow(&vec![Permission::Http(UrlPermission::Any {})]),
                false,
            );
        }
    }

    mod allow_all {
        use super::*;

        #[test]
        fn it_should_allow_any_reference() {
            run_test(
                "p1.n1.1.0.1",
                Permissions::allow(&vec![Permission::All]),
                true,
            );
        }
    }

    mod registry {
        use super::*;

        mod registry_any {
            use super::*;

            #[test]
            fn it_should_allow_any_reference() {
                run_test(
                    "p1.n1.1.0.1",
                    Permissions::allow(&vec![Permission::RegistryComponent(
                        RegistryComponentPermission {
                            publisher: None,
                            name: None,
                            version: None,
                        },
                    )]),
                    true,
                );
            }
        }

        mod registry_publisher {
            use super::*;

            fn create_permissions() -> Vec<Permission> {
                vec![Permission::RegistryComponent(RegistryComponentPermission {
                    publisher: Some("p1".to_string()),
                    name: None,
                    version: None,
                })]
            }

            #[test]
            fn it_should_allow_any_query() {
                let permissions = create_permissions();

                run_test("p1.n1.1.0.1", Permissions::allow(&permissions), true);
                run_test("p1.n2.1.0.1", Permissions::allow(&permissions), true);
                run_test("p1.n1.2.0.1", Permissions::allow(&permissions), true);
                run_test("p2.n1.1.0.1", Permissions::allow(&permissions), false);
            }
        }

        mod registry_name {
            use super::*;

            fn create_permissions() -> Vec<Permission> {
                vec![Permission::RegistryComponent(RegistryComponentPermission {
                    publisher: None,
                    name: Some("n1".to_string()),
                    version: None,
                })]
            }

            #[test]
            fn it_should_allow_any_query() {
                let permissions = create_permissions();

                run_test("p1.n1.1.0.1", Permissions::allow(&permissions), true);
                run_test("p2.n1.1.0.1", Permissions::allow(&permissions), true);
                run_test("p1.n1.2.0.1", Permissions::allow(&permissions), true);
                run_test("p1.n2.1.0.1", Permissions::allow(&permissions), false);
            }
        }

        mod registry_version {
            use super::*;

            fn create_permissions() -> Vec<Permission> {
                vec![Permission::RegistryComponent(RegistryComponentPermission {
                    publisher: None,
                    name: None,
                    version: Some(VersionReq::parse(">=1.0,<2.0").unwrap()),
                })]
            }

            #[test]
            fn it_should_allow_any_query() {
                let permissions = create_permissions();

                run_test("p1.n1.1.0.1", Permissions::allow(&permissions), true);
                run_test("p2.n1.1.0.1", Permissions::allow(&permissions), true);
                run_test("p1.n2.1.0.1", Permissions::allow(&permissions), true);
                run_test("p1.n1.1.5.2", Permissions::allow(&permissions), true);

                run_test("p1.n1.2.0.1", Permissions::allow(&permissions), false);
                run_test("p1.n1.0.0.1", Permissions::allow(&permissions), false);
            }
        }

        mod registry_all {
            use super::*;

            fn create_permissions() -> Vec<Permission> {
                vec![Permission::RegistryComponent(RegistryComponentPermission {
                    publisher: Some("p1".to_string()),
                    name: Some("n1".to_string()),
                    version: Some(VersionReq::parse(">=1.0,<2.0").unwrap()),
                })]
            }

            #[test]
            fn it_should_allow_any_query() {
                let permissions = create_permissions();

                run_test("p1.n1.1.0.1", Permissions::allow(&permissions), true);
                run_test("p1.n1.1.5.1", Permissions::allow(&permissions), true);

                run_test("p1.n1.2.0.1", Permissions::allow(&permissions), false);
                run_test("p1.n2.1.0.1", Permissions::allow(&permissions), false);
                run_test("p2.n1.1.0.1", Permissions::allow(&permissions), false);
            }
        }

        mod registry_deny {

            use super::*;

            fn create_allow_permissions() -> Vec<Permission> {
                vec![Permission::RegistryComponent(RegistryComponentPermission {
                    publisher: Some("p1".to_string()),
                    name: Some("n1".to_string()),
                    version: Some(VersionReq::parse(">=1.0,<2.0").unwrap()),
                })]
            }

            fn create_deny_permissions() -> Vec<Permission> {
                vec![Permission::RegistryComponent(RegistryComponentPermission {
                    publisher: Some("p1".to_string()),
                    name: Some("n1".to_string()),
                    version: Some(VersionReq::parse("=1.5.9").unwrap()),
                })]
            }

            #[test]
            fn it_should_deny_exact_version() {
                let allow_permissions = create_allow_permissions();
                let deny_permissions = create_deny_permissions();
                let permissions = Permissions::new(&allow_permissions, &deny_permissions);

                run_test("p1.n1.1.0.1", permissions.clone(), true);
                run_test("p1.n1.1.5.8", permissions.clone(), true);
                run_test("p1.n1.1.5.9", permissions.clone(), false);
                run_test("p1.n1.1.5.10", permissions.clone(), true);
            }
        }
    }

    mod http {
        use super::*;

        mod http_any {
            use super::*;

            #[test]
            fn it_should_allow_any_reference() {
                run_test(
                    "https://slipway-registry.com/p1/n1/1.0.1",
                    Permissions::allow(&vec![Permission::HttpComponent(UrlPermission::Any {})]),
                    true,
                );
            }
        }

        mod http_exact {
            use super::*;

            fn create_permissions() -> Vec<Permission> {
                vec![Permission::HttpComponent(UrlPermission::Exact {
                    exact: Url::parse("https://slipway-registry.com/p1/n1/1.0.1").unwrap(),
                })]
            }

            #[test]
            fn it_should_allow_exact_reference() {
                let permissions = create_permissions();

                run_test(
                    "https://slipway-registry.com/p1/n1/1.0.1",
                    Permissions::allow(&permissions),
                    true,
                );

                run_test(
                    "https://slipway-registry.com/p1/n1/2.0.1",
                    Permissions::allow(&permissions),
                    false,
                );
            }
        }

        mod http_prefix {
            use super::*;

            fn create_permissions() -> Vec<Permission> {
                vec![Permission::HttpComponent(UrlPermission::Prefix {
                    prefix: Url::parse("https://slipway-registry.com/p1/").unwrap(),
                })]
            }

            #[test]
            fn it_should_allow_prefix() {
                let permissions = create_permissions();

                run_test(
                    "https://slipway-registry.com/p1/n1/1.0.1",
                    Permissions::allow(&permissions),
                    true,
                );

                run_test(
                    "https://slipway-registry.com/p1/n2/2.0.1",
                    Permissions::allow(&permissions),
                    true,
                );

                run_test(
                    "https://slipway-registry.com/p2/n1/1.0.1",
                    Permissions::allow(&permissions),
                    false,
                );
            }
        }

        mod http_deny {
            use super::*;

            fn create_allow_permissions() -> Vec<Permission> {
                vec![Permission::HttpComponent(UrlPermission::Prefix {
                    prefix: Url::parse("https://slipway-registry.com/p1/").unwrap(),
                })]
            }

            fn create_deny_permissions() -> Vec<Permission> {
                vec![Permission::HttpComponent(UrlPermission::Prefix {
                    prefix: Url::parse("https://slipway-registry.com/p1/n2/").unwrap(),
                })]
            }

            #[test]
            fn it_should_deny_exact_version() {
                let allow_permissions = create_allow_permissions();
                let deny_permissions = create_deny_permissions();
                let permissions = Permissions::new(&allow_permissions, &deny_permissions);

                run_test(
                    "https://slipway-registry.com/p1/n1/1.0.1",
                    permissions.clone(),
                    true,
                );
                run_test(
                    "https://slipway-registry.com/p1/n2/1.0.1",
                    permissions.clone(),
                    false,
                );
                run_test(
                    "https://slipway-registry.com/p1/n3/1.0.1",
                    permissions.clone(),
                    true,
                );
            }
        }
    }

    mod local {
        use super::*;
        use slipway_engine::LocalComponentPermission;

        mod local_any {

            use super::*;

            #[test]
            fn it_should_allow_any_absolute_reference() {
                run_test(
                    "file:///path/to/component",
                    Permissions::allow(&vec![Permission::LocalComponent(
                        LocalComponentPermission::Any,
                    )]),
                    true,
                );
            }

            #[test]
            fn it_should_allow_any_local_reference() {
                run_test(
                    "file:path/to/component",
                    Permissions::allow(&vec![Permission::LocalComponent(
                        LocalComponentPermission::Any,
                    )]),
                    true,
                );
            }
        }

        mod local_exact {
            use super::*;

            fn create_permissions() -> Vec<Permission> {
                vec![Permission::LocalComponent(
                    LocalComponentPermission::Exact {
                        exact: "file:components/foo".to_string(),
                    },
                )]
            }

            #[test]
            fn it_should_allow_exact_reference() {
                let permissions = create_permissions();

                run_test(
                    "file:components/foo",
                    Permissions::allow(&permissions),
                    true,
                );

                run_test(
                    "file:components/food",
                    Permissions::allow(&permissions),
                    false,
                );

                run_test(
                    "file:///components/foo",
                    Permissions::allow(&permissions),
                    false,
                );
            }
        }

        mod local_deny {
            use super::*;

            fn create_allow_permissions() -> Vec<Permission> {
                vec![Permission::LocalComponent(
                    LocalComponentPermission::Exact {
                        exact: "file:components/foo".to_string(),
                    },
                )]
            }

            fn create_deny_permissions() -> Vec<Permission> {
                vec![Permission::LocalComponent(
                    LocalComponentPermission::Exact {
                        exact: "file:components/foo".to_string(),
                    },
                )]
            }

            #[test]
            fn it_should_deny_exact_version() {
                let allow_permissions = create_allow_permissions();
                let deny_permissions = create_deny_permissions();
                let permissions = Permissions::new(&allow_permissions, &deny_permissions);

                run_test("file:components/foo", permissions, false);
            }
        }
    }
}
