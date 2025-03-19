use std::sync::Arc;

use crate::{ComponentError, permissions::log_permissions_check};
use slipway_engine::{CallChain, ComponentExecutionContext, Permission};
use url::Url;

pub fn ensure_can_fetch_url(
    url: &Url,
    execution_context: &ComponentExecutionContext,
) -> Result<(), ComponentError> {
    log_permissions_check(&format!("fetch URL: {url}"));
    ensure_can_fetch_url_inner(url, Arc::clone(&execution_context.call_chain))
}

fn ensure_can_fetch_url_inner(
    url: &Url,
    call_chain: Arc<CallChain<'_>>,
) -> Result<(), ComponentError> {
    let is_allowed = slipway_engine::ensure_permissions(Arc::clone(&call_chain), |permissions| {
        fn matches(url: &Url, permission: &Permission) -> bool {
            match permission {
                Permission::All => true,
                Permission::Http(permission) => permission.matches(url),
                _ => false,
            }
        }

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
    });

    if !is_allowed {
        let message = format!(
            "{} does not have permission to fetch url \"{}\"",
            call_chain.rig_or_component_handle_trail_error_prefix(),
            url
        );
        return Err(super::create_permission_error(message, &call_chain));
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use slipway_engine::UrlPermission;
    use slipway_engine::{ComponentHandle, Permissions, utils::ch};

    use super::*;

    static CH: std::sync::OnceLock<ComponentHandle> = std::sync::OnceLock::new();

    fn run_test(url_str: &str, permissions: Permissions, expected: bool) {
        let url = Url::parse(url_str).unwrap();
        let handle = CH.get_or_init(|| ch("test"));
        let call_chain = Arc::new(CallChain::new_for_component(handle, permissions));
        assert_eq!(
            ensure_can_fetch_url_inner(&url, call_chain.clone()).is_ok(),
            expected
        );
    }

    mod insufficient_permissions {
        use slipway_engine::PathPermission;

        use super::*;

        #[test]
        fn it_should_forbid_any_url_for_no_permissions() {
            run_test(
                "https://example.com/foo/bar.json",
                Permissions::empty(),
                false,
            );
        }

        #[test]
        fn it_should_forbid_any_url_for_incorrect_permissions() {
            run_test(
                "https://example.com/foo/bar.json",
                Permissions::allow(&vec![Permission::File(PathPermission::Any {})]),
                false,
            );
        }
    }

    mod allow_all {
        use super::*;

        #[test]
        fn it_should_allow_any_url() {
            run_test(
                "https://example.com/foo/bar.json",
                Permissions::allow(&vec![Permission::All]),
                true,
            );
        }
    }

    mod fetch_any {
        use super::*;

        #[test]
        fn it_should_allow_any_url() {
            run_test(
                "https://example.com/foo/bar.json",
                Permissions::allow(&vec![Permission::Http(UrlPermission::Any {})]),
                true,
            );
        }
    }

    mod fetch_exact {
        use super::*;

        fn create_permissions() -> Vec<Permission> {
            vec![Permission::Http(UrlPermission::Exact {
                exact: Url::parse("https://example.com/foo/bar.json").unwrap(),
            })]
        }

        #[test]
        fn it_should_allow_exact_url() {
            run_test(
                "https://example.com/foo/bar.json",
                Permissions::allow(&create_permissions()),
                true,
            );
        }

        #[test]
        fn it_should_not_allow_different_urls() {
            let permissions = create_permissions();

            run_test(
                "https://example.com/foo/BAR.json",
                Permissions::allow(&permissions),
                false,
            );
            run_test(
                "https://example.com/foo/bar.json.exe",
                Permissions::allow(&permissions),
                false,
            );
            run_test(
                "http://example.com/foo/bar.json",
                Permissions::allow(&permissions),
                false,
            );
            run_test(
                "https://other.com/foo/bar.json",
                Permissions::allow(&permissions),
                false,
            );
        }
    }

    mod fetch_prefix {
        use super::*;

        fn create_permissions() -> Vec<Permission> {
            vec![Permission::Http(UrlPermission::Prefix {
                prefix: Url::parse("https://example.com/foo/").unwrap(),
            })]
        }

        #[test]
        fn it_should_allow_exact_url() {
            run_test(
                "https://example.com/foo/",
                Permissions::allow(&create_permissions()),
                true,
            );
        }

        #[test]
        fn it_should_allow_urls_with_prefix() {
            run_test(
                "https://example.com/foo/bar.json",
                Permissions::allow(&create_permissions()),
                true,
            );
        }

        #[test]
        fn it_should_not_allow_urls_with_other_prefix() {
            let permissions = create_permissions();

            run_test(
                "https://example.com/FOO/bar.json",
                Permissions::allow(&permissions),
                false,
            );
            run_test(
                "http://example.com/foo/bar.json",
                Permissions::allow(&permissions),
                false,
            );
            run_test(
                "https://other.com/foo/bar.json",
                Permissions::allow(&permissions),
                false,
            );
        }
    }

    mod fetch_domain {
        use super::*;

        fn create_permissions() -> Vec<Permission> {
            vec![Permission::Http(UrlPermission::Prefix {
                prefix: Url::parse("https://foo.bar.co.uk").unwrap(),
            })]
        }

        #[test]
        fn it_should_allow_exact_domain_and_scheme() {
            run_test(
                "https://foo.bar.co.uk/foo/bar.json",
                Permissions::allow(&create_permissions()),
                true,
            );
            run_test(
                "http://foo.bar.co.uk/foo/bar.json",
                Permissions::allow(&create_permissions()),
                false,
            );
        }

        #[test]
        fn it_should_not_allow_urls_with_other_domains() {
            let permissions = create_permissions();

            run_test(
                "https://example.com/foo/bar.json",
                Permissions::allow(&permissions),
                false,
            );
            run_test(
                "https://bad.foo.bar.co.uk/foo/bar.json",
                Permissions::allow(&permissions),
                false,
            );
            run_test(
                "https://moo.bar.co.uk/foo/bar.json",
                Permissions::allow(&permissions),
                false,
            );
        }
    }

    mod fetch_deny {
        use super::*;

        fn create_allow_permissions() -> Vec<Permission> {
            vec![Permission::Http(UrlPermission::Prefix {
                prefix: Url::parse("https://foo.bar.co.uk").unwrap(),
            })]
        }

        fn create_deny_permissions() -> Vec<Permission> {
            vec![Permission::Http(UrlPermission::Exact {
                exact: Url::parse("https://foo.bar.co.uk/foo/baz.json").unwrap(),
            })]
        }

        #[test]
        fn it_should_deny_specified_url() {
            run_test(
                "https://foo.bar.co.uk/foo/bar.json",
                Permissions::new(&create_allow_permissions(), &create_deny_permissions()),
                true,
            );
            run_test(
                "https://foo.bar.co.uk/foo/baz.json",
                Permissions::new(&create_allow_permissions(), &create_deny_permissions()),
                false,
            );
        }
    }

    mod fetch_mixed {
        use super::*;

        fn create_permissions() -> Vec<Permission> {
            vec![
                Permission::Http(UrlPermission::Exact {
                    exact: Url::parse("https://one.com/foo/bar.json").unwrap(),
                }),
                Permission::Http(UrlPermission::Exact {
                    exact: Url::parse("https://two.com/foo/baz.json").unwrap(),
                }),
                Permission::Http(UrlPermission::Prefix {
                    prefix: Url::parse("https://three.com/foo/").unwrap(),
                }),
                Permission::Http(UrlPermission::Prefix {
                    prefix: Url::parse("https://one.com/foo/").unwrap(),
                }),
                Permission::Http(UrlPermission::Prefix {
                    prefix: Url::parse("https://four.com").unwrap(),
                }),
                Permission::Http(UrlPermission::Prefix {
                    prefix: Url::parse("https://one.com").unwrap(),
                }),
            ]
        }

        #[test]
        fn it_should_allow_expected_urls() {
            let permissions = create_permissions();
            run_test(
                "https://one.com/foo/bar.json",
                Permissions::allow(&permissions),
                true,
            );
            run_test(
                "https://one.com/foo/baz.json",
                Permissions::allow(&permissions),
                true,
            );
            run_test(
                "https://one.com/anything.json",
                Permissions::allow(&permissions),
                true,
            );
            run_test(
                "https://two.com/foo/baz.json",
                Permissions::allow(&permissions),
                true,
            );
            run_test(
                "https://three.com/foo/whatever.json",
                Permissions::allow(&permissions),
                true,
            );
            run_test(
                "https://four.com/whatever.json",
                Permissions::allow(&permissions),
                true,
            );
        }

        #[test]
        fn it_should_not_allow_unexpected_urls() {
            let permissions = create_permissions();

            run_test(
                "https://two.com/foo/bat.json",
                Permissions::allow(&permissions),
                false,
            );
            run_test(
                "https://three.com/bar/baz.json",
                Permissions::allow(&permissions),
                false,
            );
            run_test(
                "https://five.com/foo/bar.json",
                Permissions::allow(&permissions),
                false,
            );
        }
    }
}
