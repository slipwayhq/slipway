use semver::Version;

use super::RegistryComponentPermission;

impl RegistryComponentPermission {
    pub fn matches(&self, publisher: &str, name: &str, version: &Version) -> bool {
        if let Some(required_publisher) = self.publisher.as_ref() {
            if required_publisher != publisher {
                return false;
            }
        }

        if let Some(required_name) = self.name.as_ref() {
            if required_name != name {
                return false;
            }
        }

        if let Some(required_version) = self.version.as_ref() {
            if !required_version.matches(version) {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use semver::VersionReq;

    use super::*;

    #[test]
    fn it_should_match_any() {
        let permission = RegistryComponentPermission {
            publisher: None,
            name: None,
            version: None,
        };
        assert!(permission.matches("p1", "n1", &Version::new(1, 2, 3)));
        assert!(permission.matches("p2", "n2", &Version::new(4, 5, 6)));
    }

    #[test]
    fn it_should_match_publisher() {
        let permission = RegistryComponentPermission {
            publisher: Some("p1".to_string()),
            name: None,
            version: None,
        };

        assert!(permission.matches("p1", "n1", &Version::new(1, 2, 3)));
        assert!(permission.matches("p1", "n2", &Version::new(1, 2, 3)));
        assert!(permission.matches("p1", "n1", &Version::new(4, 5, 6)));

        assert!(!permission.matches("p2", "n1", &Version::new(1, 2, 3)));
        assert!(!permission.matches("p", "n1", &Version::new(1, 2, 3)));
        assert!(!permission.matches("p11", "n1", &Version::new(1, 2, 3)));
    }

    #[test]
    fn it_should_match_name() {
        let permission = RegistryComponentPermission {
            publisher: None,
            name: Some("n1".to_string()),
            version: None,
        };

        assert!(permission.matches("p1", "n1", &Version::new(1, 2, 3)));
        assert!(permission.matches("p2", "n1", &Version::new(1, 2, 3)));
        assert!(permission.matches("p1", "n1", &Version::new(4, 5, 6)));

        assert!(!permission.matches("p1", "n2", &Version::new(1, 2, 3)));
        assert!(!permission.matches("p1", "n", &Version::new(1, 2, 3)));
        assert!(!permission.matches("p1", "n11", &Version::new(1, 2, 3)));
    }

    #[test]
    fn it_should_match_version() {
        let permission = RegistryComponentPermission {
            publisher: None,
            name: None,
            version: Some(VersionReq::parse(">=1.0,<2.0").unwrap()),
        };
        assert!(permission.matches("p1", "n1", &Version::new(1, 2, 3)));
        assert!(permission.matches("p2", "n1", &Version::new(1, 2, 3)));
        assert!(permission.matches("p1", "n2", &Version::new(1, 2, 3)));

        assert!(permission.matches("p1", "n1", &Version::new(1, 0, 0)));
        assert!(permission.matches("p1", "n1", &Version::new(1, 9, 99)));

        assert!(!permission.matches("p1", "n1", &Version::new(0, 3, 6)));
        assert!(!permission.matches("p1", "n1", &Version::new(2, 0, 0)));
        assert!(!permission.matches("p1", "n1", &Version::new(4, 5, 6)));
    }
}
