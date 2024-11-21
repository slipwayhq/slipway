use std::path::{Component, Path};

pub(super) fn is_safe_path(file_name: &Path) -> bool {
    let path = file_name.components();

    let mut depth = 0;
    for component in path {
        match component {
            Component::ParentDir => {
                if depth == 0 {
                    return false; // would escape directory
                }
                depth -= 1;
            }
            Component::Normal(_) => {
                depth += 1;
            }
            _ => {}
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_valid_paths_inside_directory() {
        assert!(is_safe_path(Path::new("file.txt")));
        assert!(is_safe_path(Path::new("subdir/file.txt")));
        assert!(is_safe_path(Path::new("./file.txt")));
        assert!(is_safe_path(Path::new("subdir/./file.txt")));
    }

    #[test]
    fn test_invalid_paths_outside_directory() {
        assert!(!is_safe_path(Path::new("../file.txt")));
        assert!(!is_safe_path(Path::new("subdir/../../file.txt")));
        assert!(!is_safe_path(Path::new("../subdir/file.txt")));
        assert!(!is_safe_path(Path::new("subdir/../../../file.txt")));
    }

    #[test]
    fn test_edge_cases() {
        // Path exactly matching the directory
        assert!(is_safe_path(Path::new("")));

        // Paths that resolve to the directory itself
        assert!(is_safe_path(Path::new(".")));
        assert!(is_safe_path(Path::new("././")));

        // Path with '..' that resolves to the directory itself
        assert!(is_safe_path(Path::new("blah/../file.txt")));

        // Path with multiple '.' and no actual navigation
        assert!(is_safe_path(Path::new("./subdir/./file.txt")));
    }
}
