use std::{path::Path, sync::Arc};

use crate::ComponentError;
use slipway_engine::{CallChain, ComponentExecutionContext, Permission};

pub fn ensure_can_fetch_file(
    path: &Path,
    execution_context: &ComponentExecutionContext,
) -> Result<(), ComponentError> {
    ensure_can_fetch_file_inner(path, Arc::clone(&execution_context.call_chain))
}

fn ensure_can_fetch_file_inner(
    path: &Path,
    call_chain: Arc<CallChain<'_>>,
) -> Result<(), ComponentError> {
    let is_allowed = slipway_engine::ensure_permissions(Arc::clone(&call_chain), |permissions| {
        fn matches(path: &Path, permission: &Permission) -> bool {
            match permission {
                Permission::All => true,
                Permission::File(permission) => permission.matches(path),
                _ => false,
            }
        }

        for permission in permissions.deny {
            if matches(path, permission) {
                super::warn_deny_permission_triggered(permission);
                return false;
            }
        }

        for permission in permissions.allow {
            if matches(path, permission) {
                return true;
            }
        }

        false
    });

    if !is_allowed {
        let message = format!(
            "{} does not have permission to fetch file \"{:?}\"",
            call_chain.rig_or_component_handle_trail_error_prefix(),
            path
        );
        return Err(super::create_permission_error(message, &call_chain));
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use common_macros::slipway_test;
    use slipway_engine::PathPermission;
    use slipway_engine::UrlPermission;
    use slipway_engine::{ComponentHandle, Permissions, utils::ch};
    use std::path::PathBuf;

    use super::*;

    static CH: std::sync::OnceLock<ComponentHandle> = std::sync::OnceLock::new();

    fn run_test(path: &Path, permissions: Permissions, expected: bool) {
        let handle = CH.get_or_init(|| ch("test"));
        let call_chain = Arc::new(CallChain::new_for_component(handle, permissions));
        assert_eq!(
            ensure_can_fetch_file_inner(path, call_chain.clone()).is_ok(),
            expected
        );
    }

    mod insufficient_permissions {

        use super::*;

        #[slipway_test]
        fn it_should_forbid_any_file_path_for_no_permissions() {
            run_test(&PathBuf::from("/foo/bar.json"), Permissions::empty(), false);
        }

        #[slipway_test]
        fn it_should_forbid_any_file_path_for_incorrect_permissions() {
            run_test(
                &PathBuf::from("/foo/bar.json"),
                Permissions::allow(&vec![Permission::Http(UrlPermission::Any {})]),
                false,
            );
        }
    }

    mod allow_all {
        use super::*;

        #[slipway_test]
        fn it_should_allow_any_file_path() {
            run_test(
                &PathBuf::from("/foo/bar.json"),
                Permissions::allow(&vec![Permission::All]),
                true,
            );
        }
    }

    mod fetch_any {

        use super::*;

        #[slipway_test]
        fn it_should_allow_any_file_path() {
            run_test(
                &PathBuf::from("/foo/bar.json"),
                Permissions::allow(&vec![Permission::File(PathPermission::Any {})]),
                true,
            );
        }
    }

    mod fetch_exact {
        use super::*;

        fn create_permissions() -> Vec<Permission> {
            vec![Permission::File(PathPermission::Exact {
                exact: PathBuf::from("/foo/bar.json"),
            })]
        }

        #[slipway_test]
        fn it_should_allow_exact_file_path() {
            run_test(
                &PathBuf::from("/foo/bar.json"),
                Permissions::allow(&create_permissions()),
                true,
            );
        }

        #[slipway_test]
        fn it_should_not_allow_different_file_paths() {
            let permissions = create_permissions();

            run_test(
                &PathBuf::from("/foo/BAR.json"),
                Permissions::allow(&permissions),
                false,
            );
            run_test(
                &PathBuf::from("/foo/bar.json.exe"),
                Permissions::allow(&permissions),
                false,
            );
            run_test(
                &PathBuf::from("foo/bar.json"),
                Permissions::allow(&permissions),
                false,
            );
        }
    }

    mod fetch_prefix {
        use super::*;

        fn create_permissions() -> Vec<Permission> {
            vec![Permission::File(PathPermission::Within {
                within: PathBuf::from("/foo/"),
            })]
        }

        #[slipway_test]
        fn it_should_allow_exact_file_path() {
            run_test(
                &PathBuf::from("/foo/"),
                Permissions::allow(&create_permissions()),
                true,
            );
        }

        #[slipway_test]
        fn it_should_allow_file_paths_with_prefix() {
            run_test(
                &PathBuf::from("/foo/bar.json"),
                Permissions::allow(&create_permissions()),
                true,
            );
        }

        #[slipway_test]
        fn it_should_allow_file_paths_with_equivalent_prefix() {
            run_test(
                &PathBuf::from("/./blah/../foo/bar.json"),
                Permissions::allow(&create_permissions()),
                true,
            );
        }
        #[slipway_test]
        fn it_should_not_allow_file_paths_with_other_prefix() {
            let permissions = create_permissions();

            run_test(
                &PathBuf::from("/FOO/bar.json"),
                Permissions::allow(&permissions),
                false,
            );
            run_test(
                &PathBuf::from("foo/bar.json"),
                Permissions::allow(&permissions),
                false,
            );
        }
    }

    mod fetch_prefix_relative {
        use super::*;

        fn create_permissions() -> Vec<Permission> {
            vec![Permission::File(PathPermission::Within {
                within: PathBuf::from("foo/"),
            })]
        }

        #[slipway_test]
        fn it_should_allow_exact_file_path() {
            run_test(
                &PathBuf::from("foo/"),
                Permissions::allow(&create_permissions()),
                true,
            );
        }

        #[slipway_test]
        fn it_should_allow_file_paths_with_prefix() {
            run_test(
                &PathBuf::from("foo/bar.json"),
                Permissions::allow(&create_permissions()),
                true,
            );
        }

        #[slipway_test]
        fn it_should_allow_file_paths_with_equivalent_prefix() {
            run_test(
                &PathBuf::from("./blah/../foo/bar.json"),
                Permissions::allow(&create_permissions()),
                true,
            );
        }

        #[slipway_test]
        fn it_should_not_allow_file_paths_with_other_prefix() {
            let permissions = create_permissions();

            run_test(
                &PathBuf::from("FOO/bar.json"),
                Permissions::allow(&permissions),
                false,
            );
            run_test(
                &PathBuf::from("/foo/bar.json"),
                Permissions::allow(&permissions),
                false,
            );
        }
    }

    mod fetch_deny {
        use super::*;

        fn create_allow_permissions() -> Vec<Permission> {
            vec![Permission::File(PathPermission::Within {
                within: PathBuf::from("/foo/bar/"),
            })]
        }

        fn create_deny_permissions() -> Vec<Permission> {
            vec![Permission::File(PathPermission::Exact {
                exact: PathBuf::from("/foo/bar/baz.json"),
            })]
        }

        #[slipway_test]
        fn it_should_deny_specified_file_path() {
            run_test(
                &PathBuf::from("/foo/bar/bar.json"),
                Permissions::new(&create_allow_permissions(), &create_deny_permissions()),
                true,
            );
            run_test(
                &PathBuf::from("/foo/bar/baz.json"),
                Permissions::new(&create_allow_permissions(), &create_deny_permissions()),
                false,
            );
        }
    }
}
