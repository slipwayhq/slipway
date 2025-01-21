use super::StringPermission;

impl StringPermission {
    pub fn matches(&self, string: &str) -> bool {
        match self {
            StringPermission::Any => true,
            StringPermission::Exact(value) => value == string,
            StringPermission::Prefix(value) => string.starts_with(value),
            StringPermission::Suffix(value) => string.ends_with(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_any_permission() {
        let permission = StringPermission::Any;
        assert!(permission.matches("anything"));
        assert!(permission.matches("something else"));
    }

    #[test]
    fn test_exact_permission() {
        let permission = StringPermission::Exact("exact".to_string());
        assert!(permission.matches("exact"));
        assert!(!permission.matches("not exact"));
        assert!(!permission.matches("exact not"));
    }

    #[test]
    fn test_prefix_permission() {
        let permission = StringPermission::Prefix("pre".to_string());
        assert!(permission.matches("pre"));
        assert!(permission.matches("prefix"));
        assert!(permission.matches("prelude blah"));
        assert!(!permission.matches("before prefix"));
        assert!(!permission.matches("pro"));
    }

    #[test]
    fn test_suffix_permission() {
        let permission = StringPermission::Suffix("fix".to_string());
        assert!(permission.matches("fix"));
        assert!(permission.matches("postfix"));
        assert!(!permission.matches("suffix after"));
    }
}
