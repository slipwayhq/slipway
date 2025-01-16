use std::sync::Arc;

use slipway_engine::{CallChain, ComponentExecutionContext, ComponentPermission};
use url::Url;

use crate::ComponentError;

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
    if !slipway_engine::ensure_permissions(Arc::clone(&call_chain), |permissions| {
        for permission in permissions {
            match permission {
                ComponentPermission::FetchAny | ComponentPermission::FullTrust => return true,
                ComponentPermission::FetchExact { url } => {
                    if url == url_str {
                        return true;
                    }
                }
                ComponentPermission::FetchPrefix { prefix } => {
                    if url_str.starts_with(prefix) {
                        return true;
                    }
                }
                ComponentPermission::FetchDomain { domain } => {
                    if url.domain() == Some(domain) {
                        return true;
                    }
                }

                _ => {}
            }
        }
        false
    }) {
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
    use slipway_engine::{utils::ch, ComponentHandle};

    use super::*;

    static CH: std::sync::OnceLock<ComponentHandle> = std::sync::OnceLock::new();

    fn run_test(url_str: &str, permissions: &Vec<ComponentPermission>, expected: bool) {
        let url = Url::parse(url_str).unwrap();
        let handle = CH.get_or_init(|| ch("test"));
        let call_chain = Arc::new(CallChain::new_for_component(handle, permissions));
        assert_eq!(
            ensure_can_fetch_url_inner(url_str, &url, call_chain.clone()).is_ok(),
            expected
        );
    }

    mod insufficient_permissions {
        use super::*;

        #[test]
        fn it_should_forbid_any_url_for_no_permissions() {
            run_test("https://example.com/foo/bar.json", &vec![], false);
        }

        #[test]
        fn it_should_forbid_any_url_for_incorrect_permissions() {
            run_test(
                "https://example.com/foo/bar.json",
                &vec![ComponentPermission::Noop],
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
                &vec![ComponentPermission::FullTrust],
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
                &vec![ComponentPermission::FetchAny],
                true,
            );
        }
    }

    mod fetch_exact {
        use super::*;

        fn create_permissions() -> Vec<ComponentPermission> {
            vec![ComponentPermission::FetchExact {
                url: "https://example.com/foo/bar.json".to_string(),
            }]
        }

        #[test]
        fn it_should_allow_exact_url() {
            run_test(
                "https://example.com/foo/bar.json",
                &create_permissions(),
                true,
            );
        }

        #[test]
        fn it_should_not_allow_different_urls() {
            let permissions = create_permissions();

            run_test("https://example.com/foo/BAR.json", &permissions, false);
            run_test("https://example.com/foo/bar.json.exe", &permissions, false);
            run_test("http://example.com/foo/bar.json", &permissions, false);
            run_test("https://other.com/foo/bar.json", &permissions, false);
        }
    }

    mod fetch_prefix {
        use super::*;

        fn create_permissions() -> Vec<ComponentPermission> {
            vec![ComponentPermission::FetchPrefix {
                prefix: "https://example.com/foo/".to_string(),
            }]
        }

        #[test]
        fn it_should_allow_exact_url() {
            run_test("https://example.com/foo/", &create_permissions(), true);
        }

        #[test]
        fn it_should_allow_urls_with_prefix() {
            run_test(
                "https://example.com/foo/bar.json",
                &create_permissions(),
                true,
            );
        }

        #[test]
        fn it_should_not_allow_urls_with_other_prefix() {
            let permissions = create_permissions();

            run_test("https://example.com/FOO/bar.json", &permissions, false);
            run_test("http://example.com/foo/bar.json", &permissions, false);
            run_test("https://other.com/foo/bar.json", &permissions, false);
        }
    }

    mod fetch_domain {
        use super::*;

        fn create_permissions() -> Vec<ComponentPermission> {
            vec![ComponentPermission::FetchDomain {
                domain: "foo.bar.co.uk".to_string(),
            }]
        }

        #[test]
        fn it_should_allow_exact_domain() {
            run_test(
                "https://foo.bar.co.uk/foo/bar.json",
                &create_permissions(),
                true,
            );
            run_test(
                "http://foo.bar.co.uk/foo/bar.json",
                &create_permissions(),
                true,
            );
        }

        #[test]
        fn it_should_not_allow_urls_with_other_domains() {
            let permissions = create_permissions();

            run_test("https://example.com/foo/bar.json", &permissions, false);
            run_test(
                "https://bad.foo.bar.co.uk/foo/bar.json",
                &permissions,
                false,
            );
            run_test("https://moo.bar.co.uk/foo/bar.json", &permissions, false);
        }
    }

    mod fetch_mixed {
        use super::*;

        fn create_permissions() -> Vec<ComponentPermission> {
            vec![
                ComponentPermission::FetchExact {
                    url: "https://one.com/foo/bar.json".to_string(),
                },
                ComponentPermission::FetchExact {
                    url: "https://two.com/foo/baz.json".to_string(),
                },
                ComponentPermission::FetchPrefix {
                    prefix: "https://three.com/foo/".to_string(),
                },
                ComponentPermission::FetchPrefix {
                    prefix: "https://one.com/foo/".to_string(),
                },
                ComponentPermission::FetchDomain {
                    domain: "four.com".to_string(),
                },
                ComponentPermission::FetchDomain {
                    domain: "one.com".to_string(),
                },
            ]
        }

        #[test]
        fn it_should_allow_expected_urls() {
            let permissions = create_permissions();
            run_test("https://one.com/foo/bar.json", &permissions, true);
            run_test("https://one.com/foo/baz.json", &permissions, true);
            run_test("https://one.com/anything.json", &permissions, true);
            run_test("https://two.com/foo/baz.json", &permissions, true);
            run_test("https://three.com/foo/whatever.json", &permissions, true);
            run_test("https://four.com/whatever.json", &permissions, true);
        }

        #[test]
        fn it_should_not_allow_unexpected_urls() {
            let permissions = create_permissions();

            run_test("https://two.com/foo/bat.json", &permissions, false);
            run_test("https://three.com/bar/baz.json", &permissions, false);
            run_test("https://five.com/foo/bar.json", &permissions, false);
        }
    }
}
