use std::path::Path;

use super::PathPermission;
use normalize_path::NormalizePath;

impl PathPermission {
    pub fn matches(&self, path: &Path) -> bool {
        match self {
            PathPermission::Any {} => true,
            PathPermission::Exact { exact } => exact.normalize() == path.normalize(),
            PathPermission::Within { within: prefix } => {
                println!(
                    "{} starts with {} ?",
                    path.normalize().to_string_lossy(),
                    prefix.normalize().to_string_lossy()
                );
                path.normalize().starts_with(prefix.normalize())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use common_macros::slipway_test;

    use super::*;

    #[slipway_test]
    fn test_any_permission() {
        let permission = PathPermission::Any {};
        assert!(permission.matches(&PathBuf::from("anything")));
        assert!(permission.matches(&PathBuf::from("something/else")));
    }

    #[slipway_test]
    fn test_exact_permission() {
        let permission = PathPermission::Exact {
            exact: PathBuf::from("exact/file.txt"),
        };
        assert!(permission.matches(&PathBuf::from("exact/file.txt")));
        assert!(!permission.matches(&PathBuf::from("/exact/file.txt")));
        assert!(!permission.matches(&PathBuf::from("file.txt")));
    }

    #[slipway_test]
    fn test_prefix_permission() {
        let permission = PathPermission::Within {
            within: PathBuf::from("some/prefix"),
        };
        assert!(permission.matches(&PathBuf::from("some/prefix")));
        assert!(permission.matches(&PathBuf::from("some/prefix/blah")));
        assert!(!permission.matches(&PathBuf::from("some/prefix_2")));
        assert!(!permission.matches(&PathBuf::from("not/some/prefix")));
        assert!(!permission.matches(&PathBuf::from("prefix")));
    }
}
