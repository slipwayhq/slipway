use std::sync::Arc;

use crate::ComponentError;
use slipway_engine::{CallChain, ComponentExecutionContext, Permission, UrlPermission};
use url::Url;

pub fn ensure_can_fetch_url(
    url_str: &str,
    url: &Url,
    execution_context: &ComponentExecutionContext,
) -> Result<(), ComponentError> {
    ensure_can_fetch_url_inner(url_str, url, Arc::clone(&execution_context.call_chain))
}

pub fn ensure_can_fetch_url_inner(
    url_str: &str,
    url: &Url,
    call_chain: Arc<CallChain<'_>>,
) -> Result<(), ComponentError> {
    let is_allowed = slipway_engine::ensure_permissions(Arc::clone(&call_chain), |permissions| {
        fn matches(url_str: &str, url: &Url, permission: &Permission) -> bool {
            match permission {
                Permission::HttpFetch(UrlPermission::Any) | Permission::All => true,
                Permission::HttpFetch(UrlPermission::Exact { url }) => url == url_str,
                Permission::HttpFetch(UrlPermission::Prefix { prefix }) => {
                    url_str.starts_with(prefix)
                }
                Permission::HttpFetch(UrlPermission::Domain { domain }) => {
                    url.domain() == Some(domain)
                }

                _ => false,
            }
        }

        for permission in permissions.deny {
            if matches(url_str, url, permission) {
                return false;
            }
        }

        for permission in permissions.allow {
            if matches(url_str, url, permission) {
                return true;
            }
        }

        false
    });

    if !is_allowed {
        return Err(ComponentError {
            message: format!(
                "Component {} does not have permission to fetch url {}",
                call_chain.component_handle_trail(),
                url
            ),
            inner: vec![],
        });
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use slipway_engine::{utils::ch, ComponentHandle, Permissions};

    use super::*;

    static CH: std::sync::OnceLock<ComponentHandle> = std::sync::OnceLock::new();

    fn run_test(url_str: &str, permissions: Permissions, expected: bool) {
        let url = Url::parse(url_str).unwrap();
        let handle = CH.get_or_init(|| ch("test"));
        let call_chain = Arc::new(CallChain::new_for_component(handle, permissions));
        assert_eq!(
            ensure_can_fetch_url_inner(url_str, &url, call_chain.clone()).is_ok(),
            expected
        );
    }

    mod insufficient_permissions {
        use slipway_engine::StringPermission;

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
                Permissions::allow(&vec![Permission::FontQuery(StringPermission::Any)]),
                false,
            );
        }
    }

    mod full_trust {
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
                Permissions::allow(&vec![Permission::HttpFetch(UrlPermission::Any)]),
                true,
            );
        }
    }

    mod fetch_exact {
        use super::*;

        fn create_permissions() -> Vec<Permission> {
            vec![Permission::HttpFetch(UrlPermission::Exact {
                url: "https://example.com/foo/bar.json".to_string(),
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
            vec![Permission::HttpFetch(UrlPermission::Prefix {
                prefix: "https://example.com/foo/".to_string(),
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
            vec![Permission::HttpFetch(UrlPermission::Domain {
                domain: "foo.bar.co.uk".to_string(),
            })]
        }

        #[test]
        fn it_should_allow_exact_domain() {
            run_test(
                "https://foo.bar.co.uk/foo/bar.json",
                Permissions::allow(&create_permissions()),
                true,
            );
            run_test(
                "http://foo.bar.co.uk/foo/bar.json",
                Permissions::allow(&create_permissions()),
                true,
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

    mod deny {
        use super::*;

        fn create_allow_permissions() -> Vec<Permission> {
            vec![Permission::HttpFetch(UrlPermission::Domain {
                domain: "foo.bar.co.uk".to_string(),
            })]
        }

        fn create_deny_permissions() -> Vec<Permission> {
            vec![Permission::HttpFetch(UrlPermission::Exact {
                url: "https://foo.bar.co.uk/foo/baz.json".to_string(),
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
                Permission::HttpFetch(UrlPermission::Exact {
                    url: "https://one.com/foo/bar.json".to_string(),
                }),
                Permission::HttpFetch(UrlPermission::Exact {
                    url: "https://two.com/foo/baz.json".to_string(),
                }),
                Permission::HttpFetch(UrlPermission::Prefix {
                    prefix: "https://three.com/foo/".to_string(),
                }),
                Permission::HttpFetch(UrlPermission::Prefix {
                    prefix: "https://one.com/foo/".to_string(),
                }),
                Permission::HttpFetch(UrlPermission::Domain {
                    domain: "four.com".to_string(),
                }),
                Permission::HttpFetch(UrlPermission::Domain {
                    domain: "one.com".to_string(),
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
