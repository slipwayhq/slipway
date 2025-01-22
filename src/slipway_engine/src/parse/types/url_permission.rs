use url::Url;

use super::UrlPermission;

impl UrlPermission {
    pub fn matches(&self, url: &Url) -> bool {
        match self {
            UrlPermission::Any {} => true,
            UrlPermission::Exact { exact } => exact.as_str() == url.as_str(),
            UrlPermission::Prefix { prefix } => url.as_str().starts_with(prefix.as_str()),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    #[test]
    fn it_should_match_any_url() {
        let permission = UrlPermission::Any {};
        assert!(permission.matches(&url("https://example.com")));
        assert!(permission.matches(&url("http://other.co.uk/foo.bar.json")));
    }

    #[test]
    fn it_should_match_exact_domain() {
        let permission = UrlPermission::Exact {
            exact: url("https://example.com"),
        };

        assert!(permission.matches(&url("https://example.com")));
        assert!(permission.matches(&url("https://example.com/")));

        assert!(!permission.matches(&url("http://example.com")));
        assert!(!permission.matches(&url("https://example.com/index.html")));
        assert!(!permission.matches(&url("https://example.comm")));
        assert!(!permission.matches(&url("https://example.com/other")));
        assert!(!permission.matches(&url("https://example.org")));
    }

    #[test]
    fn it_should_match_exact_domain_with_path() {
        let permission = UrlPermission::Exact {
            exact: url("https://example.com/some/file.json"),
        };

        assert!(permission.matches(&url("https://example.com/some/file.json")));

        assert!(!permission.matches(&url("https://example.com/some/file.json?query=1")));
        assert!(!permission.matches(&url("http://example.com/some/file.json")));
        assert!(!permission.matches(&url("https://example.com")));
        assert!(!permission.matches(&url("http://example.com/some/file.json.exe")));
        assert!(!permission.matches(&url("http://example.com/some/file.json/other.txt")));
        assert!(!permission.matches(&url("http://example.com/some/file.json/")));
    }

    #[test]
    fn it_should_match_prefix() {
        let permission = UrlPermission::Prefix {
            prefix: url("https://example.com/some/file.json"),
        };

        assert!(permission.matches(&url("https://example.com/some/file.json")));
        assert!(permission.matches(&url("https://example.com/some/file.json?query=1")));
        assert!(permission.matches(&url("https://example.com/some/file.json/other.txt")));

        assert!(!permission.matches(&url("http://example.com/some/file.json")));
        assert!(!permission.matches(&url("https://example.com")));
    }
}
