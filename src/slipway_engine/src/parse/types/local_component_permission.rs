use super::LocalComponentPermission;

impl LocalComponentPermission {
    pub fn matches(&self, path: &str) -> bool {
        match self {
            LocalComponentPermission::Any {} => true,
            LocalComponentPermission::Exact { exact } => exact == path,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_match_any() {
        let permission = LocalComponentPermission::Any {};
        assert!(permission.matches("file:blah"));
        assert!(permission.matches("file:///some/path.tar"));
    }

    #[test]
    fn it_should_match_exact() {
        let permission = LocalComponentPermission::Exact {
            exact: "file:///some/path.tar".to_string(),
        };
        assert!(permission.matches("file:///some/path.tar"));
        assert!(!permission.matches("file:some/path.tar"));
        assert!(!permission.matches("file:///some/path.tar.gs"));
        assert!(!permission.matches("file:///some/path"));
    }
}
