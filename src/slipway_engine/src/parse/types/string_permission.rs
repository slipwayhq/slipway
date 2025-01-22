use super::StringPermission;

impl StringPermission {
    pub fn matches(&self, string: &str) -> bool {
        match self {
            StringPermission::Any {} => true,
            StringPermission::Exact { exact } => exact == string,
            StringPermission::Prefix { prefix } => string.starts_with(prefix),
            StringPermission::Suffix { suffix } => string.ends_with(suffix),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_any_permission() {
        let permission = StringPermission::Any {};
        assert!(permission.matches("anything"));
        assert!(permission.matches("something else"));
    }

    #[test]
    fn test_exact_permission() {
        let permission = StringPermission::Exact {
            exact: "exact".to_string(),
        };
        assert!(permission.matches("exact"));
        assert!(!permission.matches("not exact"));
        assert!(!permission.matches("exact not"));
    }

    #[test]
    fn test_prefix_permission() {
        let permission = StringPermission::Prefix {
            prefix: "pre".to_string(),
        };
        assert!(permission.matches("pre"));
        assert!(permission.matches("prefix"));
        assert!(permission.matches("prelude blah"));
        assert!(!permission.matches("before prefix"));
        assert!(!permission.matches("pro"));
    }

    #[test]
    fn test_suffix_permission() {
        let permission = StringPermission::Suffix {
            suffix: "fix".to_string(),
        };
        assert!(permission.matches("fix"));
        assert!(permission.matches("postfix"));
        assert!(!permission.matches("suffix after"));
    }
}
