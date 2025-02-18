use std::{borrow::Cow, path::PathBuf};

use regex::Regex;
use std::sync::LazyLock;
use tracing::warn;
use url::Url;

#[derive(Debug, Eq, PartialEq)]
pub enum ProcessedUrl {
    RelativePath(PathBuf),
    AbsolutePath(PathBuf),
    Http(Url),
}

static RELATIVE_FILE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^file:(?<path>[^/].*)$").unwrap());

const INCORRECT_FILE_PREFIX: &str = "file://";
const CORRECT_FILE_PREFIX: &str = "file:///";

pub fn process_url_str(url_str: &str) -> Result<ProcessedUrl, String> {
    if let Some(caps) = RELATIVE_FILE_REGEX.captures(url_str) {
        let relative_path_str = &caps["path"];
        let path = PathBuf::from(relative_path_str);
        return Ok(ProcessedUrl::RelativePath(path));
    }

    let url_str = if !url_str.starts_with(CORRECT_FILE_PREFIX) {
        if let Some(rest) = url_str.strip_prefix(INCORRECT_FILE_PREFIX) {
            warn!("Replacing {INCORRECT_FILE_PREFIX} with {CORRECT_FILE_PREFIX} in url: {url_str}");
            Cow::Owned(format!("{CORRECT_FILE_PREFIX}{}", rest))
        } else {
            Cow::Borrowed(url_str)
        }
    } else {
        Cow::Borrowed(url_str)
    };

    let url = Url::parse(url_str.as_ref())
        .map_err(|e| format!("Failed to parse url \"{}\". Error: {}", url_str, e))?;

    match url.scheme() {
        "file" => {
            let file_path = url
                .to_file_path()
                .map_err(|_| format!("unable to convert URL to local path: {url}"))?;

            Ok(ProcessedUrl::AbsolutePath(file_path))
        }
        "https" | "http" => Ok(ProcessedUrl::Http(url)),
        other => Err(format!("unsupported URL scheme: {other}")),
    }
}

#[cfg(test)]
mod test {
    use url::Url;

    #[test]
    fn it_should_process_relative_paths() {
        assert_eq!(
            super::process_url_str("file:./test.txt").unwrap(),
            super::ProcessedUrl::RelativePath(std::path::PathBuf::from("./test.txt"))
        );

        assert_eq!(
            super::process_url_str("file:some/folder/test.txt").unwrap(),
            super::ProcessedUrl::RelativePath(std::path::PathBuf::from("some/folder/test.txt"))
        );

        assert_eq!(
            super::process_url_str("file:some/folder").unwrap(),
            super::ProcessedUrl::RelativePath(std::path::PathBuf::from("some/folder"))
        );
    }

    #[test]
    fn it_should_process_absolute_paths() {
        assert_eq!(
            super::process_url_str("file:///test.txt").unwrap(),
            super::ProcessedUrl::AbsolutePath(std::path::PathBuf::from("/test.txt"))
        );

        assert_eq!(
            super::process_url_str("file:///some/folder/test.txt").unwrap(),
            super::ProcessedUrl::AbsolutePath(std::path::PathBuf::from("/some/folder/test.txt"))
        );

        assert_eq!(
            super::process_url_str("file:///some/folder").unwrap(),
            super::ProcessedUrl::AbsolutePath(std::path::PathBuf::from("/some/folder"))
        );
    }

    #[test]
    fn it_should_fix_absolute_paths_with_host() {
        assert_eq!(
            super::process_url_str("file://test.txt").unwrap(),
            super::ProcessedUrl::AbsolutePath(std::path::PathBuf::from("/test.txt"))
        );

        assert_eq!(
            super::process_url_str("file://some/folder/test.txt").unwrap(),
            super::ProcessedUrl::AbsolutePath(std::path::PathBuf::from("/some/folder/test.txt"))
        );

        assert_eq!(
            super::process_url_str("file://some/folder").unwrap(),
            super::ProcessedUrl::AbsolutePath(std::path::PathBuf::from("/some/folder"))
        );
    }

    #[test]
    fn it_should_process_http_urls() {
        assert_eq!(
            super::process_url_str("http://blah.com/test.txt").unwrap(),
            super::ProcessedUrl::Http(Url::parse("http://blah.com/test.txt").unwrap())
        );
    }

    #[test]
    fn it_should_process_https_urls() {
        assert_eq!(
            super::process_url_str("https://blah.com/test.txt").unwrap(),
            super::ProcessedUrl::Http(Url::parse("https://blah.com/test.txt").unwrap())
        );
    }

    #[test]
    fn it_should_return_error_on_invalid_urls() {
        let maybe_result = super::process_url_str("https://b::*.com/test.txt");
        assert!(maybe_result.is_err());
    }

    #[test]
    fn it_should_return_error_on_unsupported_schemes() {
        let maybe_result = super::process_url_str("ftp://blah.com/test.txt");
        assert!(maybe_result.is_err());
    }
}
