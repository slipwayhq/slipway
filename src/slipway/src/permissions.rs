use anyhow::Context;
use clap::Args;
use paste::paste;
use slipway_engine::{
    LocalComponentPermission, Permission, RegistryComponentPermission, StringPermission,
    UrlPermission,
};
use url::Url;

#[derive(Debug)]
pub(super) struct Permissions {
    pub allow: Vec<Permission>,
    pub deny: Vec<Permission>,
}

impl<'a> From<&'a Permissions> for slipway_engine::Permissions<'a> {
    fn from(permissions: &'a Permissions) -> slipway_engine::Permissions<'a> {
        slipway_engine::Permissions {
            allow: &permissions.allow,
            deny: &permissions.deny,
        }
    }
}

macro_rules! create_url_permissions {
    ($permission_name:ident, $name:ident, $doc_name:expr) => {
        paste! {
            #[derive(Debug, Args)]
            pub(super) struct $permission_name {
                #[doc = "Allow any " $doc_name "s at the rig level."]
                #[arg(long)]
                [<allow_ $name>]: bool,

                #[doc = "Allow a specific " $doc_name " at the rig level."]
                #[arg(long)]
                [<allow_ $name _exact>]: Vec<url::Url>,

                #[doc = "Allow " $doc_name "s with the given prefix at the rig level."]
                #[arg(long)]
                [<allow_ $name _prefix>]: Vec<url::Url>,

                #[doc = "Deny any " $doc_name "s at the rig level."]
                #[arg(long)]
                [<deny_ $name>]: bool,

                #[doc = "Deny a specific " $doc_name " at the rig level."]
                #[arg(long)]
                [<deny_ $name _exact>]: Vec<url::Url>,

                #[doc = "Deny " $doc_name "s with the given prefix at the rig level."]
                #[arg(long)]
                [<deny_ $name _prefix>]: Vec<url::Url>,
            }
        }
    };
}

macro_rules! create_string_permissions {
    ($permission_name:ident, $name:ident, $doc_name:expr) => {
        paste! {
            #[derive(Debug, Args)]
            pub(super) struct $permission_name {
                #[doc = "Allow any " $doc_name "s at the rig level."]
                #[arg(long)]
                [<allow_ $name>]: bool,

                #[doc = "Allow a specific " $doc_name " at the rig level."]
                #[arg(long)]
                [<allow_ $name _exact>]: Vec<String>,

                #[doc = "Allow " $doc_name "s with the given prefix at the rig level."]
                #[arg(long)]
                [<allow_ $name _prefix>]: Vec<String>,

                #[doc = "Allow " $doc_name "s with the given suffix at the rig level."]
                #[arg(long)]
                [<allow_ $name _suffix>]: Vec<String>,

                #[doc = "Deny any " $doc_name "s at the rig level."]
                #[arg(long)]
                [<deny_ $name>]: bool,

                #[doc = "Deny a specific " $doc_name " at the rig level."]
                #[arg(long)]
                [<deny_ $name _exact>]: Vec<String>,

                #[doc = "Deny " $doc_name "s with the given prefix at the rig level."]
                #[arg(long)]
                [<deny_ $name _prefix>]: Vec<String>,

                #[doc = "Deny " $doc_name "s with the given suffix at the rig level."]
                #[arg(long)]
                [<deny_ $name _suffix>]: Vec<String>,
            }
        }
    };
}

macro_rules! create_simple_string_permissions {
    ($permission_name:ident, $name:ident, $doc_name:expr) => {
        paste! {
            #[derive(Debug, Args)]
            pub(super) struct $permission_name {
                #[doc = "Allow any " $doc_name "s at the rig level."]
                #[arg(long)]
                [<allow_ $name>]: bool,

                #[doc = "Allow a specific " $doc_name " at the rig level."]
                #[arg(long)]
                [<allow_ $name _exact>]: Vec<String>,

                #[doc = "Deny any " $doc_name "s at the rig level."]
                #[arg(long)]
                [<deny_ $name>]: bool,

                #[doc = "Deny a specific " $doc_name " at the rig level."]
                #[arg(long)]
                [<deny_ $name _exact>]: Vec<String>,
            }
        }
    };
}

create_url_permissions!(HttpPermissionArgs, http, "HTTP request");
create_string_permissions!(FontPermissionArgs, fonts, "font");
create_string_permissions!(EnvPermissionArgs, env, "environment variable");
create_url_permissions!(
    HttpComponentPermissionArgs,
    http_components,
    "HTTP component"
);
create_simple_string_permissions!(
    LocalComponentPermissionArgs,
    local_components,
    "local component"
);

#[derive(Debug, Args)]
pub(super) struct RegistryComponentPermissionArgs {
    /// Allow any registry components at the rig level.
    #[arg(long)]
    allow_registry_components: bool,

    /// Allow the rig to use specific registry components.
    /// This should be a string of the form "publisher.name.version_spec".
    /// Each part is optional, but the dots must be present.
    /// For example:
    ///   "slipwayhq.render.1.0.0"
    ///   "slipwayhq.render.>=1.0.0,<2.0.0"
    ///   "slipwayhq.render."
    ///   "slipwayhq.."
    ///   ".render."
    #[arg(long, verbatim_doc_comment)]
    allow_registry_components_exact: Vec<String>,

    /// Deny any registry components at the rig level.
    #[arg(long)]
    deny_registry_components: bool,

    /// Deny the rig to use specific registry components.
    /// See the corresponding allow permission for formatting.
    #[arg(long, verbatim_doc_comment)]
    deny_registry_components_exact: Vec<String>,
}

#[derive(Debug, Args)]
pub(super) struct CommonPermissionsArgs {
    /// Allow all permissions at the rig level.
    #[arg(long)]
    allow_all: bool,

    /// Deny all permissions at the rig level.
    #[arg(long)]
    deny_all: bool,

    #[command(flatten)]
    http: HttpPermissionArgs,

    #[command(flatten)]
    font: FontPermissionArgs,

    #[command(flatten)]
    env: EnvPermissionArgs,

    #[command(flatten)]
    http_components: HttpComponentPermissionArgs,

    #[command(flatten)]
    local_components: LocalComponentPermissionArgs,

    #[command(flatten)]
    registry_components: RegistryComponentPermissionArgs,
}

impl CommonPermissionsArgs {
    pub fn into_permissions(self) -> anyhow::Result<Permissions> {
        let mut allow = Vec::new();
        let mut deny = Vec::new();

        if self.allow_all {
            allow.push(Permission::All);
        }

        if self.deny_all {
            deny.push(Permission::All);
        }

        // Http
        add_url_permissions(
            &mut allow,
            &mut deny,
            Permission::Http,
            self.http.allow_http,
            self.http.allow_http_exact,
            self.http.allow_http_prefix,
            self.http.deny_http,
            self.http.deny_http_exact,
            self.http.deny_http_prefix,
        );

        // Fonts
        add_string_permissions(
            &mut allow,
            &mut deny,
            Permission::Font,
            self.font.allow_fonts,
            self.font.allow_fonts_exact,
            self.font.allow_fonts_prefix,
            self.font.allow_fonts_suffix,
            self.font.deny_fonts,
            self.font.deny_fonts_exact,
            self.font.deny_fonts_prefix,
            self.font.deny_fonts_suffix,
        );

        // Env
        add_string_permissions(
            &mut allow,
            &mut deny,
            Permission::Env,
            self.env.allow_env,
            self.env.allow_env_exact,
            self.env.allow_env_prefix,
            self.env.allow_env_suffix,
            self.env.deny_env,
            self.env.deny_env_exact,
            self.env.deny_env_prefix,
            self.env.deny_env_suffix,
        );

        // Http Components
        add_url_permissions(
            &mut allow,
            &mut deny,
            Permission::HttpComponent,
            self.http_components.allow_http_components,
            self.http_components.allow_http_components_exact,
            self.http_components.allow_http_components_prefix,
            self.http_components.deny_http_components,
            self.http_components.deny_http_components_exact,
            self.http_components.deny_http_components_prefix,
        );

        // Local Components
        add_local_component_permissions(
            &mut allow,
            &mut deny,
            self.local_components.allow_local_components,
            self.local_components.allow_local_components_exact,
            self.local_components.deny_local_components,
            self.local_components.deny_local_components_exact,
        );

        // Registry Components
        add_registry_component_permissions(
            &mut allow,
            &mut deny,
            self.registry_components.allow_registry_components,
            self.registry_components.allow_registry_components_exact,
            self.registry_components.deny_registry_components,
            self.registry_components.deny_registry_components_exact,
        )?;

        Ok(Permissions { allow, deny })
    }
}

#[allow(clippy::too_many_arguments)]
fn add_url_permissions<F>(
    allow_list: &mut Vec<Permission>,
    deny_list: &mut Vec<Permission>,
    variant_ctor: F,
    allow_any: bool,
    allow_exact: Vec<Url>,
    allow_prefix: Vec<Url>,
    deny_any: bool,
    deny_exact: Vec<Url>,
    deny_prefix: Vec<Url>,
) where
    F: Fn(UrlPermission) -> Permission,
{
    if allow_any {
        allow_list.push(variant_ctor(UrlPermission::Any {}));
    }
    for exact in allow_exact {
        allow_list.push(variant_ctor(UrlPermission::Exact { exact }));
    }
    for prefix in allow_prefix {
        allow_list.push(variant_ctor(UrlPermission::Prefix { prefix }));
    }

    if deny_any {
        deny_list.push(variant_ctor(UrlPermission::Any {}));
    }
    for exact in deny_exact {
        deny_list.push(variant_ctor(UrlPermission::Exact { exact }));
    }
    for prefix in deny_prefix {
        deny_list.push(variant_ctor(UrlPermission::Prefix { prefix }));
    }
}

#[allow(clippy::too_many_arguments)]
fn add_string_permissions<F>(
    allow_list: &mut Vec<Permission>,
    deny_list: &mut Vec<Permission>,
    variant_ctor: F,
    allow_any: bool,
    allow_exact: Vec<String>,
    allow_prefix: Vec<String>,
    allow_suffix: Vec<String>,
    deny_any: bool,
    deny_exact: Vec<String>,
    deny_prefix: Vec<String>,
    deny_suffix: Vec<String>,
) where
    F: Fn(StringPermission) -> Permission,
{
    if allow_any {
        allow_list.push(variant_ctor(StringPermission::Any {}));
    }
    for exact in allow_exact {
        allow_list.push(variant_ctor(StringPermission::Exact { exact }));
    }
    for prefix in allow_prefix {
        allow_list.push(variant_ctor(StringPermission::Prefix { prefix }));
    }
    for suffix in allow_suffix {
        allow_list.push(variant_ctor(StringPermission::Suffix { suffix }));
    }

    if deny_any {
        deny_list.push(variant_ctor(StringPermission::Any {}));
    }
    for exact in deny_exact {
        deny_list.push(variant_ctor(StringPermission::Exact { exact }));
    }
    for prefix in deny_prefix {
        deny_list.push(variant_ctor(StringPermission::Prefix { prefix }));
    }
    for suffix in deny_suffix {
        deny_list.push(variant_ctor(StringPermission::Suffix { suffix }));
    }
}

// Local components are only "Any" or "Exact".
fn add_local_component_permissions(
    allow_list: &mut Vec<Permission>,
    deny_list: &mut Vec<Permission>,
    allow_any: bool,
    allow_exact: Vec<String>,
    deny_any: bool,
    deny_exact: Vec<String>,
) {
    if allow_any {
        allow_list.push(Permission::LocalComponent(LocalComponentPermission::Any));
    }
    for exact in allow_exact {
        allow_list.push(Permission::LocalComponent(
            LocalComponentPermission::Exact { exact },
        ));
    }

    if deny_any {
        deny_list.push(Permission::LocalComponent(LocalComponentPermission::Any));
    }
    for exact in deny_exact {
        deny_list.push(Permission::LocalComponent(
            LocalComponentPermission::Exact { exact },
        ));
    }
}

// Registry components need some parsing logic to map "publisher/name@version" into fields.
fn add_registry_component_permissions(
    allow_list: &mut Vec<Permission>,
    deny_list: &mut Vec<Permission>,
    allow_any: bool,
    allow_exact: Vec<String>,
    deny_any: bool,
    deny_exact: Vec<String>,
) -> anyhow::Result<()> {
    if allow_any {
        allow_list.push(Permission::RegistryComponent(RegistryComponentPermission {
            publisher: None,
            name: None,
            version: None,
        }));
    }

    for item in allow_exact {
        allow_list.push(Permission::RegistryComponent(
            parse_registry_component_permission(&item)?,
        ));
    }

    if deny_any {
        deny_list.push(Permission::RegistryComponent(RegistryComponentPermission {
            publisher: None,
            name: None,
            version: None,
        }));
    }

    for item in deny_exact {
        deny_list.push(Permission::RegistryComponent(
            parse_registry_component_permission(&item)?,
        ));
    }

    Ok(())
}

pub fn parse_registry_component_permission(s: &str) -> anyhow::Result<RegistryComponentPermission> {
    // Split into 3 parts
    let mut parts = s.splitn(3, '.');
    let publisher_part = parts.next().unwrap();
    let name_part = parts.next().unwrap();
    let version_part = parts.next().unwrap();

    // Convert publisher and name to Option<String>
    let publisher = if !publisher_part.is_empty() {
        Some(publisher_part.to_string())
    } else {
        None
    };

    let name = if !name_part.is_empty() {
        Some(name_part.to_string())
    } else {
        None
    };

    // Parse version if provided
    let version = if !version_part.is_empty() {
        Some(semver::VersionReq::parse(version_part).with_context(|| {
            format!(
                "Failed to parse version requirement from: '{}'",
                version_part
            )
        })?)
    } else {
        None
    };

    Ok(RegistryComponentPermission {
        publisher,
        name,
        version,
    })
}

#[cfg(test)]
mod tests {

    use super::*;
    use semver::VersionReq;
    use url::Url;

    #[test]
    fn test_allow_all() {
        let args = CommonPermissionsArgs {
            allow_all: true,
            deny_all: false,
            http: HttpPermissionArgs {
                allow_http: true,
                allow_http_exact: vec![],
                allow_http_prefix: vec![],
                deny_http: false,
                deny_http_exact: vec![],
                deny_http_prefix: vec![],
            },
            font: FontPermissionArgs {
                allow_fonts: true,
                allow_fonts_exact: vec![],
                allow_fonts_prefix: vec![],
                allow_fonts_suffix: vec![],
                deny_fonts: false,
                deny_fonts_exact: vec![],
                deny_fonts_prefix: vec![],
                deny_fonts_suffix: vec![],
            },
            env: EnvPermissionArgs {
                allow_env: true,
                allow_env_exact: vec![],
                allow_env_prefix: vec![],
                allow_env_suffix: vec![],
                deny_env: false,
                deny_env_exact: vec![],
                deny_env_prefix: vec![],
                deny_env_suffix: vec![],
            },
            http_components: HttpComponentPermissionArgs {
                allow_http_components: true,
                allow_http_components_exact: vec![],
                allow_http_components_prefix: vec![],
                deny_http_components: false,
                deny_http_components_exact: vec![],
                deny_http_components_prefix: vec![],
            },
            local_components: LocalComponentPermissionArgs {
                allow_local_components: true,
                allow_local_components_exact: vec![],
                deny_local_components: false,
                deny_local_components_exact: vec![],
            },
            registry_components: RegistryComponentPermissionArgs {
                allow_registry_components: true,
                allow_registry_components_exact: vec![],
                deny_registry_components: false,
                deny_registry_components_exact: vec![],
            },
        };

        let permissions = args.into_permissions().unwrap();

        assert_eq!(
            permissions.allow,
            vec![
                Permission::All,
                Permission::Http(UrlPermission::Any {}),
                Permission::Font(StringPermission::Any {}),
                Permission::Env(StringPermission::Any {}),
                Permission::HttpComponent(UrlPermission::Any {}),
                Permission::LocalComponent(LocalComponentPermission::Any {}),
                Permission::RegistryComponent(RegistryComponentPermission {
                    publisher: None,
                    name: None,
                    version: None,
                }),
            ]
        );
        assert!(permissions.deny.is_empty());
    }

    #[test]
    fn test_deny_all() {
        let args = CommonPermissionsArgs {
            allow_all: false,
            deny_all: true,
            http: HttpPermissionArgs {
                allow_http: false,
                allow_http_exact: vec![],
                allow_http_prefix: vec![],
                deny_http: true,
                deny_http_exact: vec![],
                deny_http_prefix: vec![],
            },
            font: FontPermissionArgs {
                allow_fonts: false,
                allow_fonts_exact: vec![],
                allow_fonts_prefix: vec![],
                allow_fonts_suffix: vec![],
                deny_fonts: true,
                deny_fonts_exact: vec![],
                deny_fonts_prefix: vec![],
                deny_fonts_suffix: vec![],
            },
            env: EnvPermissionArgs {
                allow_env: false,
                allow_env_exact: vec![],
                allow_env_prefix: vec![],
                allow_env_suffix: vec![],
                deny_env: true,
                deny_env_exact: vec![],
                deny_env_prefix: vec![],
                deny_env_suffix: vec![],
            },
            http_components: HttpComponentPermissionArgs {
                allow_http_components: false,
                allow_http_components_exact: vec![],
                allow_http_components_prefix: vec![],
                deny_http_components: true,
                deny_http_components_exact: vec![],
                deny_http_components_prefix: vec![],
            },
            local_components: LocalComponentPermissionArgs {
                allow_local_components: false,
                allow_local_components_exact: vec![],
                deny_local_components: true,
                deny_local_components_exact: vec![],
            },
            registry_components: RegistryComponentPermissionArgs {
                allow_registry_components: false,
                allow_registry_components_exact: vec![],
                deny_registry_components: true,
                deny_registry_components_exact: vec![],
            },
        };

        let permissions = args.into_permissions().unwrap();

        assert!(permissions.allow.is_empty());
        assert_eq!(
            permissions.deny,
            vec![
                Permission::All,
                Permission::Http(UrlPermission::Any {}),
                Permission::Font(StringPermission::Any {}),
                Permission::Env(StringPermission::Any {}),
                Permission::HttpComponent(UrlPermission::Any {}),
                Permission::LocalComponent(LocalComponentPermission::Any {}),
                Permission::RegistryComponent(RegistryComponentPermission {
                    publisher: None,
                    name: None,
                    version: None,
                }),
            ]
        );
    }

    #[test]
    fn test_http_permissions() {
        let args = CommonPermissionsArgs {
            allow_all: false,
            deny_all: false,
            http: HttpPermissionArgs {
                allow_http: true,
                allow_http_exact: vec![Url::parse("https://example.com").unwrap()],
                allow_http_prefix: vec![Url::parse("https://example2.com").unwrap()],
                deny_http: true,
                deny_http_exact: vec![Url::parse("https://example3.com").unwrap()],
                deny_http_prefix: vec![Url::parse("https://example4.com").unwrap()],
            },
            font: FontPermissionArgs {
                allow_fonts: false,
                allow_fonts_exact: vec![],
                allow_fonts_prefix: vec![],
                allow_fonts_suffix: vec![],
                deny_fonts: false,
                deny_fonts_exact: vec![],
                deny_fonts_prefix: vec![],
                deny_fonts_suffix: vec![],
            },
            env: EnvPermissionArgs {
                allow_env: false,
                allow_env_exact: vec![],
                allow_env_prefix: vec![],
                allow_env_suffix: vec![],
                deny_env: false,
                deny_env_exact: vec![],
                deny_env_prefix: vec![],
                deny_env_suffix: vec![],
            },
            http_components: HttpComponentPermissionArgs {
                allow_http_components: false,
                allow_http_components_exact: vec![],
                allow_http_components_prefix: vec![],
                deny_http_components: false,
                deny_http_components_exact: vec![],
                deny_http_components_prefix: vec![],
            },
            local_components: LocalComponentPermissionArgs {
                allow_local_components: false,
                allow_local_components_exact: vec![],
                deny_local_components: false,
                deny_local_components_exact: vec![],
            },
            registry_components: RegistryComponentPermissionArgs {
                allow_registry_components: false,
                allow_registry_components_exact: vec![],
                deny_registry_components: false,
                deny_registry_components_exact: vec![],
            },
        };

        let permissions = args.into_permissions().unwrap();

        assert_eq!(
            permissions.allow,
            vec![
                Permission::Http(UrlPermission::Any {}),
                Permission::Http(UrlPermission::Exact {
                    exact: Url::parse("https://example.com").unwrap()
                }),
                Permission::Http(UrlPermission::Prefix {
                    prefix: Url::parse("https://example2.com").unwrap()
                }),
            ]
        );

        assert_eq!(
            permissions.deny,
            vec![
                Permission::Http(UrlPermission::Any {}),
                Permission::Http(UrlPermission::Exact {
                    exact: Url::parse("https://example3.com").unwrap()
                }),
                Permission::Http(UrlPermission::Prefix {
                    prefix: Url::parse("https://example4.com").unwrap()
                }),
            ]
        );
    }

    #[test]
    fn test_string_permissions_for_fonts() {
        let args = CommonPermissionsArgs {
            allow_all: false,
            deny_all: false,
            http: HttpPermissionArgs {
                allow_http: false,
                allow_http_exact: vec![],
                allow_http_prefix: vec![],
                deny_http: false,
                deny_http_exact: vec![],
                deny_http_prefix: vec![],
            },
            font: FontPermissionArgs {
                allow_fonts: true,
                allow_fonts_exact: vec!["Roboto".to_string()],
                allow_fonts_prefix: vec!["Hack".to_string()],
                allow_fonts_suffix: vec!["Arial".to_string()],
                deny_fonts: true,
                deny_fonts_exact: vec!["Foo".to_string()],
                deny_fonts_prefix: vec!["Bar".to_string()],
                deny_fonts_suffix: vec!["Baz".to_string()],
            },
            env: EnvPermissionArgs {
                allow_env: false,
                allow_env_exact: vec![],
                allow_env_prefix: vec![],
                allow_env_suffix: vec![],
                deny_env: false,
                deny_env_exact: vec![],
                deny_env_prefix: vec![],
                deny_env_suffix: vec![],
            },
            http_components: HttpComponentPermissionArgs {
                allow_http_components: false,
                allow_http_components_exact: vec![],
                allow_http_components_prefix: vec![],
                deny_http_components: false,
                deny_http_components_exact: vec![],
                deny_http_components_prefix: vec![],
            },
            local_components: LocalComponentPermissionArgs {
                allow_local_components: false,
                allow_local_components_exact: vec![],
                deny_local_components: false,
                deny_local_components_exact: vec![],
            },
            registry_components: RegistryComponentPermissionArgs {
                allow_registry_components: false,
                allow_registry_components_exact: vec![],
                deny_registry_components: false,
                deny_registry_components_exact: vec![],
            },
        };

        let permissions = args.into_permissions().unwrap();

        assert_eq!(
            permissions.allow,
            vec![
                Permission::Font(StringPermission::Any {}),
                Permission::Font(StringPermission::Exact {
                    exact: "Roboto".into()
                }),
                Permission::Font(StringPermission::Prefix {
                    prefix: "Hack".into()
                }),
                Permission::Font(StringPermission::Suffix {
                    suffix: "Arial".into()
                }),
            ]
        );
        assert_eq!(
            permissions.deny,
            vec![
                Permission::Font(StringPermission::Any {}),
                Permission::Font(StringPermission::Exact {
                    exact: "Foo".into()
                }),
                Permission::Font(StringPermission::Prefix {
                    prefix: "Bar".into()
                }),
                Permission::Font(StringPermission::Suffix {
                    suffix: "Baz".into()
                }),
            ]
        );
    }

    #[test]
    fn test_string_permissions_for_env() {
        let args = CommonPermissionsArgs {
            allow_all: false,
            deny_all: false,
            http: HttpPermissionArgs {
                allow_http: false,
                allow_http_exact: vec![],
                allow_http_prefix: vec![],
                deny_http: false,
                deny_http_exact: vec![],
                deny_http_prefix: vec![],
            },
            font: FontPermissionArgs {
                allow_fonts: false,
                allow_fonts_exact: vec![],
                allow_fonts_prefix: vec![],
                allow_fonts_suffix: vec![],
                deny_fonts: false,
                deny_fonts_exact: vec![],
                deny_fonts_prefix: vec![],
                deny_fonts_suffix: vec![],
            },
            env: EnvPermissionArgs {
                allow_env: true,
                allow_env_exact: vec!["One".to_string()],
                allow_env_prefix: vec!["Two".to_string()],
                allow_env_suffix: vec!["Three".to_string()],
                deny_env: true,
                deny_env_exact: vec!["Foo".to_string()],
                deny_env_prefix: vec!["Bar".to_string()],
                deny_env_suffix: vec!["Baz".to_string()],
            },
            http_components: HttpComponentPermissionArgs {
                allow_http_components: false,
                allow_http_components_exact: vec![],
                allow_http_components_prefix: vec![],
                deny_http_components: false,
                deny_http_components_exact: vec![],
                deny_http_components_prefix: vec![],
            },
            local_components: LocalComponentPermissionArgs {
                allow_local_components: false,
                allow_local_components_exact: vec![],
                deny_local_components: false,
                deny_local_components_exact: vec![],
            },
            registry_components: RegistryComponentPermissionArgs {
                allow_registry_components: false,
                allow_registry_components_exact: vec![],
                deny_registry_components: false,
                deny_registry_components_exact: vec![],
            },
        };

        let permissions = args.into_permissions().unwrap();

        assert_eq!(
            permissions.allow,
            vec![
                Permission::Env(StringPermission::Any {}),
                Permission::Env(StringPermission::Exact {
                    exact: "One".into()
                }),
                Permission::Env(StringPermission::Prefix {
                    prefix: "Two".into()
                }),
                Permission::Env(StringPermission::Suffix {
                    suffix: "Three".into()
                }),
            ]
        );
        assert_eq!(
            permissions.deny,
            vec![
                Permission::Env(StringPermission::Any {}),
                Permission::Env(StringPermission::Exact {
                    exact: "Foo".into()
                }),
                Permission::Env(StringPermission::Prefix {
                    prefix: "Bar".into()
                }),
                Permission::Env(StringPermission::Suffix {
                    suffix: "Baz".into()
                }),
            ]
        );
    }

    #[test]
    fn test_http_component_permissions() {
        let args = CommonPermissionsArgs {
            allow_all: false,
            deny_all: false,
            http: HttpPermissionArgs {
                allow_http: false,
                allow_http_exact: vec![],
                allow_http_prefix: vec![],
                deny_http: false,
                deny_http_exact: vec![],
                deny_http_prefix: vec![],
            },
            font: FontPermissionArgs {
                allow_fonts: false,
                allow_fonts_exact: vec![],
                allow_fonts_prefix: vec![],
                allow_fonts_suffix: vec![],
                deny_fonts: false,
                deny_fonts_exact: vec![],
                deny_fonts_prefix: vec![],
                deny_fonts_suffix: vec![],
            },
            env: EnvPermissionArgs {
                allow_env: false,
                allow_env_exact: vec![],
                allow_env_prefix: vec![],
                allow_env_suffix: vec![],
                deny_env: false,
                deny_env_exact: vec![],
                deny_env_prefix: vec![],
                deny_env_suffix: vec![],
            },
            http_components: HttpComponentPermissionArgs {
                allow_http_components: true,
                allow_http_components_exact: vec![Url::parse("https://example.com").unwrap()],
                allow_http_components_prefix: vec![Url::parse("https://example2.com").unwrap()],
                deny_http_components: true,
                deny_http_components_exact: vec![Url::parse("https://example3.com").unwrap()],
                deny_http_components_prefix: vec![Url::parse("https://example4.com").unwrap()],
            },
            local_components: LocalComponentPermissionArgs {
                allow_local_components: false,
                allow_local_components_exact: vec![],
                deny_local_components: false,
                deny_local_components_exact: vec![],
            },
            registry_components: RegistryComponentPermissionArgs {
                allow_registry_components: false,
                allow_registry_components_exact: vec![],
                deny_registry_components: false,
                deny_registry_components_exact: vec![],
            },
        };

        let permissions = args.into_permissions().unwrap();

        assert_eq!(
            permissions.allow,
            vec![
                Permission::HttpComponent(UrlPermission::Any {}),
                Permission::HttpComponent(UrlPermission::Exact {
                    exact: Url::parse("https://example.com").unwrap()
                }),
                Permission::HttpComponent(UrlPermission::Prefix {
                    prefix: Url::parse("https://example2.com").unwrap()
                }),
            ]
        );

        assert_eq!(
            permissions.deny,
            vec![
                Permission::HttpComponent(UrlPermission::Any {}),
                Permission::HttpComponent(UrlPermission::Exact {
                    exact: Url::parse("https://example3.com").unwrap()
                }),
                Permission::HttpComponent(UrlPermission::Prefix {
                    prefix: Url::parse("https://example4.com").unwrap()
                }),
            ]
        );
    }

    #[test]
    fn test_local_component_permissions() {
        let args = CommonPermissionsArgs {
            allow_all: false,
            deny_all: false,
            http: HttpPermissionArgs {
                allow_http: false,
                allow_http_exact: vec![],
                allow_http_prefix: vec![],
                deny_http: false,
                deny_http_exact: vec![],
                deny_http_prefix: vec![],
            },
            font: FontPermissionArgs {
                allow_fonts: false,
                allow_fonts_exact: vec![],
                allow_fonts_prefix: vec![],
                allow_fonts_suffix: vec![],
                deny_fonts: false,
                deny_fonts_exact: vec![],
                deny_fonts_prefix: vec![],
                deny_fonts_suffix: vec![],
            },
            env: EnvPermissionArgs {
                allow_env: false,
                allow_env_exact: vec![],
                allow_env_prefix: vec![],
                allow_env_suffix: vec![],
                deny_env: false,
                deny_env_exact: vec![],
                deny_env_prefix: vec![],
                deny_env_suffix: vec![],
            },
            http_components: HttpComponentPermissionArgs {
                allow_http_components: false,
                allow_http_components_exact: vec![],
                allow_http_components_prefix: vec![],
                deny_http_components: false,
                deny_http_components_exact: vec![],
                deny_http_components_prefix: vec![],
            },
            local_components: LocalComponentPermissionArgs {
                allow_local_components: true,
                allow_local_components_exact: vec!["foo.wasm".to_string()],
                deny_local_components: true,
                deny_local_components_exact: vec!["bar.wasm".to_string()],
            },
            registry_components: RegistryComponentPermissionArgs {
                allow_registry_components: false,
                allow_registry_components_exact: vec![],
                deny_registry_components: false,
                deny_registry_components_exact: vec![],
            },
        };

        let permissions = args.into_permissions().unwrap();

        assert_eq!(
            permissions.allow,
            vec![
                Permission::LocalComponent(LocalComponentPermission::Any),
                Permission::LocalComponent(LocalComponentPermission::Exact {
                    exact: "foo.wasm".into()
                }),
            ]
        );

        assert_eq!(
            permissions.deny,
            vec![
                Permission::LocalComponent(LocalComponentPermission::Any),
                Permission::LocalComponent(LocalComponentPermission::Exact {
                    exact: "bar.wasm".into()
                }),
            ]
        );
    }

    #[test]
    fn test_registry_components() {
        let args = CommonPermissionsArgs {
            allow_all: false,
            deny_all: false,
            http: HttpPermissionArgs {
                allow_http: false,
                allow_http_exact: vec![],
                allow_http_prefix: vec![],
                deny_http: false,
                deny_http_exact: vec![],
                deny_http_prefix: vec![],
            },
            font: FontPermissionArgs {
                allow_fonts: false,
                allow_fonts_exact: vec![],
                allow_fonts_prefix: vec![],
                allow_fonts_suffix: vec![],
                deny_fonts: false,
                deny_fonts_exact: vec![],
                deny_fonts_prefix: vec![],
                deny_fonts_suffix: vec![],
            },
            env: EnvPermissionArgs {
                allow_env: false,
                allow_env_exact: vec![],
                allow_env_prefix: vec![],
                allow_env_suffix: vec![],
                deny_env: false,
                deny_env_exact: vec![],
                deny_env_prefix: vec![],
                deny_env_suffix: vec![],
            },
            http_components: HttpComponentPermissionArgs {
                allow_http_components: false,
                allow_http_components_exact: vec![],
                allow_http_components_prefix: vec![],
                deny_http_components: false,
                deny_http_components_exact: vec![],
                deny_http_components_prefix: vec![],
            },
            local_components: LocalComponentPermissionArgs {
                allow_local_components: false,
                allow_local_components_exact: vec![],
                deny_local_components: false,
                deny_local_components_exact: vec![],
            },
            registry_components: RegistryComponentPermissionArgs {
                allow_registry_components: true,
                allow_registry_components_exact: vec!["foo.bar.1.2.3".to_string()],
                deny_registry_components: true,
                deny_registry_components_exact: vec![".baz.1.0.0".to_string()],
            },
        };

        let permissions = args.into_permissions().unwrap();

        assert_eq!(
            permissions.allow,
            vec![
                Permission::RegistryComponent(RegistryComponentPermission {
                    publisher: None,
                    name: None,
                    version: None,
                }),
                Permission::RegistryComponent(RegistryComponentPermission {
                    publisher: Some("foo".into()),
                    name: Some("bar".into()),
                    version: Some(VersionReq::parse("1.2.3").unwrap()),
                })
            ]
        );

        assert_eq!(
            permissions.deny,
            vec![
                Permission::RegistryComponent(RegistryComponentPermission {
                    publisher: None,
                    name: None,
                    version: None,
                }),
                Permission::RegistryComponent(RegistryComponentPermission {
                    publisher: None,
                    name: Some("baz".into()),
                    version: Some(VersionReq::parse("1.0.0").unwrap()),
                })
            ]
        );
    }

    #[test]
    fn test_parse_registry_component_permission() {
        let parsed = parse_registry_component_permission("publisher.name.>=2.0.0,<3.0.0").unwrap();
        assert_eq!(parsed.publisher, Some("publisher".into()));
        assert_eq!(parsed.name, Some("name".into()));
        assert_eq!(
            parsed.version,
            Some(VersionReq::parse(">=2.0.0,<3.0.0").unwrap())
        );

        let parsed_no_version = parse_registry_component_permission("publisher.other.").unwrap();
        assert_eq!(parsed_no_version.publisher, Some("publisher".into()));
        assert_eq!(parsed_no_version.name, Some("other".into()));
        assert_eq!(parsed_no_version.version, None);

        let parsed_just_name = parse_registry_component_permission(".only_name.").unwrap();
        assert_eq!(parsed_just_name.publisher, None);
        assert_eq!(parsed_just_name.name, Some("only_name".into()));
        assert_eq!(parsed_just_name.version, None);

        let parsed_just_publisher =
            parse_registry_component_permission("only_publisher..").unwrap();
        assert_eq!(
            parsed_just_publisher.publisher,
            Some("only_publisher".into())
        );
        assert_eq!(parsed_just_publisher.name, None);
        assert_eq!(parsed_just_publisher.version, None);

        let parsed_just_version = parse_registry_component_permission("..1.2.3").unwrap();
        assert_eq!(parsed_just_version.publisher, None);
        assert_eq!(parsed_just_version.name, None);
        assert_eq!(
            parsed_just_version.version,
            Some(VersionReq::parse("1.2.3").unwrap())
        );
    }
}
