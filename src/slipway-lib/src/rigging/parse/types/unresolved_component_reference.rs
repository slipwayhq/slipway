use crate::errors::SlipwayError;
use once_cell::sync::Lazy;
use regex::Regex;
use semver::VersionReq;
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::{fmt::Display, path::PathBuf, str::FromStr};
use url::Url;

const COMPONENT_REFERENCE_REGISTRY_OWNER_SEPARATOR: char = '.';
const COMPONENT_REFERENCE_GIT_USER_SEPARATOR: char = '/';
const COMPONENT_REFERENCE_VERSION_SEPARATOR: char = '#';

const ROOT_REFERENCE: &str = ".root";

pub(crate) static REGISTRY_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(?<owner>[\w-]+)\.(?<name>[\w-]+)#(?<version>.+)$").unwrap());

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum UnresolvedComponentReference {
    // .root
    Root,

    // owner.id#version
    Registry {
        owner: String,
        name: String,
        version: VersionReq,
    },

    // user/string#1.0
    GitHub {
        user: String,
        repository: String,
        version: GitHubVersion,
    },

    // file://path
    Local {
        path: PathBuf,
    },

    // https://url
    Url {
        url: Url,
    },
}

impl FromStr for UnresolvedComponentReference {
    type Err = SlipwayError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == ROOT_REFERENCE {
            return Ok(UnresolvedComponentReference::Root);
        }

        if let Some(caps) = REGISTRY_REGEX.captures(s) {
            let version = parse_version_requirement(&caps["version"])?;

            return Ok(UnresolvedComponentReference::Registry {
                owner: caps["owner"].to_string(),
                name: caps["name"].to_string(),
                version,
            });
        }

        static GIT_REGEX: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"^(?<user>[\w-]+)/(?<repository>[\w-]+)#(?<version>.+)$").unwrap()
        });
        if let Some(caps) = GIT_REGEX.captures(s) {
            let version = GitHubVersion::from_str(&caps["version"])?;

            return Ok(UnresolvedComponentReference::GitHub {
                user: caps["user"].to_string(),
                repository: caps["repository"].to_string(),
                version,
            });
        }

        if let Ok(uri) = Url::parse(s) {
            return match uri.scheme() {
                "file" => Ok(UnresolvedComponentReference::Local {
                    path: uri.to_file_path().expect("URI was not a valid file path"),
                }),
                "https" => Ok(UnresolvedComponentReference::Url { url: uri }),
                other => Err(SlipwayError::InvalidComponentReference(format!(
                    "unsupported URI scheme: {other}"
                ))),
            };
        }

        Err(SlipwayError::InvalidComponentReference(
            "component reference was not in a valid format".to_string(),
        ))
    }
}

fn parse_version_requirement(version_string: &str) -> Result<VersionReq, SlipwayError> {
    let Ok(version) = VersionReq::parse(version_string) else {
        return Err(SlipwayError::InvalidComponentReference(
            "version requirement was not in a valid format".to_string(),
        ));
    };
    Ok(version)
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GitHubVersion {
    Commitish(String),
    Version(VersionReq),
}

impl FromStr for GitHubVersion {
    type Err = SlipwayError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const SEMVER_PREFIX: &str = "semver:";
        if let Some(semver) = s.strip_prefix(SEMVER_PREFIX) {
            let version = parse_version_requirement(semver)?;
            return Ok(GitHubVersion::Version(version));
        }

        Ok(GitHubVersion::Commitish(s.to_string()))
    }
}

impl UnresolvedComponentReference {
    pub fn is_root(&self) -> bool {
        match self {
            UnresolvedComponentReference::Root => true,
            _ => false,
        }
    }

    #[cfg(test)]
    pub fn exact(id: &str, version: &str) -> Self {
        UnresolvedComponentReference::Registry {
            owner: "".to_string(),
            name: id.to_string(),
            version: VersionReq::parse(version).expect("Invalid version"),
        }
    }
}

impl Display for GitHubVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitHubVersion::Commitish(commit) => f.write_str(commit),
            GitHubVersion::Version(version) => {
                f.write_fmt(format_args!("{}{}", "semver:", version))
            }
        }
    }
}

impl Display for UnresolvedComponentReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnresolvedComponentReference::Root => f.write_str(".root"),
            UnresolvedComponentReference::Registry {
                owner,
                name,
                version,
            } => f.write_fmt(format_args!(
                "{}{}{}{}{}",
                owner,
                COMPONENT_REFERENCE_REGISTRY_OWNER_SEPARATOR,
                name,
                COMPONENT_REFERENCE_VERSION_SEPARATOR,
                version
            )),
            UnresolvedComponentReference::GitHub {
                user,
                repository,
                version,
            } => f.write_fmt(format_args!(
                "{}{}{}{}{}",
                user,
                COMPONENT_REFERENCE_GIT_USER_SEPARATOR,
                repository,
                COMPONENT_REFERENCE_VERSION_SEPARATOR,
                version
            )),
            UnresolvedComponentReference::Local { path } => {
                f.write_fmt(format_args!("{}", path.to_string_lossy()))
            }
            UnresolvedComponentReference::Url { url } => f.write_fmt(format_args!("{}", url)),
        }
    }
}

impl<'de> Deserialize<'de> for UnresolvedComponentReference {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        match value.as_str() {
            Some(reference_as_string) => {
                UnresolvedComponentReference::from_str(reference_as_string)
                    .map_err(serde::de::Error::custom)
            }

            None => Err(serde::de::Error::custom(
                "reference should be in string format",
            )),
        }
    }
}

#[cfg(test)]
mod root_tests {
    use super::*;

    #[test]
    fn it_should_parse_root() {
        let s = r".root";

        let reference = UnresolvedComponentReference::from_str(s).unwrap();

        let UnresolvedComponentReference::Root = reference else {
            panic!("Unexpected unresolved reference: {reference:?}");
        };
    }
}

#[cfg(test)]
mod registry_tests {
    use super::*;

    #[test]
    fn it_should_deserialize_unresolved_component_reference_from_string() {
        let json = r#""test-owner.test-name#1.2.3""#;

        let reference: UnresolvedComponentReference = serde_json::from_str(json).unwrap();

        let UnresolvedComponentReference::Registry { owner, name, version } = reference else {
            panic!("Unexpected unresolved reference: {reference:?}");
        };

        assert_eq!(owner, "test-owner");
        assert_eq!(name, "test-name");
        assert_eq!(version, VersionReq::parse("1.2.3").unwrap());
    }

    #[test]
    fn it_should_parse_unresolved_component_reference_from_string() {
        let s = r"test-owner.test-name#1.2.3";

        let reference = UnresolvedComponentReference::from_str(s).unwrap();

        let UnresolvedComponentReference::Registry { owner, name, version } = reference else {
            panic!("Unexpected unresolved reference: {reference:?}");
        };

        assert_eq!(owner, "test-owner");
        assert_eq!(name, "test-name");
        assert_eq!(version, VersionReq::parse("1.2.3").unwrap());
    }

    #[test]
    fn it_should_parse_unresolved_component_reference_from_string_with_short_version() {
        let s = r"test-owner.test-name#1";

        let reference = UnresolvedComponentReference::from_str(s).unwrap();

        let UnresolvedComponentReference::Registry { owner, name, version } = reference else {
            panic!("Unexpected unresolved reference: {reference:?}");
        };

        assert_eq!(owner, "test-owner");
        assert_eq!(name, "test-name");
        assert_eq!(version, VersionReq::parse("1").unwrap());
    }

    #[test]
    fn it_should_fail_to_parse_unresolved_component_reference_from_string_if_no_version() {
        let s = "test-owner.test-name";

        let reference_result = UnresolvedComponentReference::from_str(s);

        assert!(reference_result.is_err());
    }

    #[test]
    fn it_should_fail_to_parse_unresolved_component_reference_from_string_if_empty_version() {
        let s = "test-owner.test-name#";

        let reference_result = UnresolvedComponentReference::from_str(s);

        assert!(reference_result.is_err());
    }

    #[test]
    fn it_should_fail_to_parse_unresolved_component_reference_from_string_if_no_owner() {
        let s = "test-name#1.2.3";

        let reference_result = UnresolvedComponentReference::from_str(s);

        assert!(reference_result.is_err());
    }

    #[test]
    fn it_should_fail_to_parse_unresolved_component_reference_from_string_if_empty_owner() {
        let s = ".test-name#1.2.3";

        let reference_result = UnresolvedComponentReference::from_str(s);

        assert!(reference_result.is_err());
    }
}

#[cfg(test)]
mod github_tests {
    use super::*;

    #[test]
    fn it_should_parse_unresolved_component_reference_from_string() {
        let s = r"test-user/test-repository#semver:1.2.3";

        let reference = UnresolvedComponentReference::from_str(s).unwrap();

        let UnresolvedComponentReference::GitHub { user, repository, version } = reference else {
            panic!("Unexpected unresolved reference: {reference:?}");
        };

        assert_eq!(user, "test-user");
        assert_eq!(repository, "test-repository");
        assert_eq!(
            version,
            GitHubVersion::Version(VersionReq::parse("1.2.3").unwrap())
        );
    }

    #[test]
    fn it_should_parse_unresolved_component_reference_from_string_with_short_version() {
        let s = r"test-user/test-repository#semver:1";

        let reference = UnresolvedComponentReference::from_str(s).unwrap();

        let UnresolvedComponentReference::GitHub { user, repository, version } = reference else {
            panic!("Unexpected unresolved reference: {reference:?}");
        };

        assert_eq!(user, "test-user");
        assert_eq!(repository, "test-repository");
        assert_eq!(
            version,
            GitHubVersion::Version(VersionReq::parse("1").unwrap())
        );
    }
    #[test]
    fn it_should_parse_unresolved_component_reference_with_commitish_from_string() {
        let s = r"test-user/test-repository#blah";

        let reference = UnresolvedComponentReference::from_str(s).unwrap();

        let UnresolvedComponentReference::GitHub { user, repository, version } = reference else {
            panic!("Unexpected unresolved reference: {reference:?}");
        };

        assert_eq!(user, "test-user");
        assert_eq!(repository, "test-repository");
        assert_eq!(version, GitHubVersion::Commitish("blah".to_string()));
    }

    #[test]
    fn it_should_fail_to_parse_unresolved_component_reference_from_string_if_no_version() {
        let s = "test-user/test-repository";

        let reference_result = UnresolvedComponentReference::from_str(s);

        assert!(reference_result.is_err());
    }

    #[test]
    fn it_should_fail_to_parse_unresolved_component_reference_from_string_if_empty_version() {
        let s = "test-user/test-repository#";

        let reference_result = UnresolvedComponentReference::from_str(s);

        assert!(reference_result.is_err());
    }
}

#[cfg(test)]
mod local_tests {
    use super::*;

    #[test]
    fn it_should_parse_local_files() {
        let uri = r"file:///usr/local/rigging.json";

        let reference = UnresolvedComponentReference::from_str(uri).unwrap();

        let UnresolvedComponentReference::Local { path } = reference else {
            panic!("Unexpected unresolved reference: {reference:?}");
        };

        assert_eq!(path, PathBuf::from_str("/usr/local/rigging.json").unwrap());
    }
}

#[cfg(test)]
mod url_tests {
    use super::*;

    #[test]
    fn it_should_parse_urls() {
        let uri = r"https://asdf.com/asdf.tar.gz";

        let reference = UnresolvedComponentReference::from_str(uri).unwrap();

        let UnresolvedComponentReference::Url { url } = reference else {
            panic!("Unexpected unresolved reference: {reference:?}");
        };

        assert_eq!(url, Url::parse(uri).unwrap());
    }
}
