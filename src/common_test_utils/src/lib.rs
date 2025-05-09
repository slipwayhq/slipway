use std::path::{Path, PathBuf};

pub mod test_server;

pub const SLIPWAY_TEST_COMPONENTS_PATH: &str = "./components";

pub const SLIPWAY_INCREMENT_COMPONENT_NAME: &str = "slipwayhq.increment.0.0.1";
pub const SLIPWAY_INCREMENT_COMPONENT_FOLDER_NAME: &str = "slipwayhq.increment";
pub const SLIPWAY_INCREMENT_COMPONENT_TAR_NAME: &str = "slipwayhq.increment.0.0.1.tar";
pub const SLIPWAY_INCREMENT_JS_COMPONENT_TAR_NAME: &str = "slipwayhq.increment_js.0.0.1.tar";
pub const SLIPWAY_INCREMENT_INVALID_CALLOUT_PERMISSIONS_COMPONENT_TAR_NAME: &str =
    "slipwayhq.increment_invalid_callout_permissions.0.0.1.tar";
pub const SLIPWAY_INCREMENT_JS_INVALID_CALLOUT_PERMISSIONS_COMPONENT_TAR_NAME: &str =
    "slipwayhq.increment_js_invalid_callout_permissions.0.0.1.tar";

pub const SLIPWAY_FETCH_COMPONENT_NAME: &str = "slipwayhq.fetch.0.0.1";
pub const SLIPWAY_FETCH_COMPONENT_TAR_NAME: &str = "slipwayhq.fetch.0.0.1.tar";
pub const SLIPWAY_FETCH_JS_COMPONENT_TAR_NAME: &str = "slipwayhq.fetch_js.0.0.1.tar";
pub const SLIPWAY_FETCH_ERROR_JS_COMPONENT_TAR_NAME: &str = "slipwayhq.fetch_error_js.0.0.1.tar";

pub const SLIPWAY_COMPONENT_FILE_COMPONENT_NAME: &str = "slipwayhq.component_file.0.0.1";
pub const SLIPWAY_COMPONENT_FILE_COMPONENT_TAR_NAME: &str = "slipwayhq.component_file.0.0.1.tar";
pub const SLIPWAY_COMPONENT_FILE_JS_COMPONENT_TAR_NAME: &str =
    "slipwayhq.component_file_js.0.0.1.tar";

pub const SLIPWAY_FONT_COMPONENT_NAME: &str = "slipwayhq.font.0.0.1";
pub const SLIPWAY_FONT_COMPONENT_TAR_NAME: &str = "slipwayhq.font.0.0.1.tar";
pub const SLIPWAY_FONT_JS_COMPONENT_TAR_NAME: &str = "slipwayhq.font_js.0.0.1.tar";

pub const SLIPWAY_ENV_COMPONENT_NAME: &str = "slipwayhq.env.0.0.1";
pub const SLIPWAY_ENV_COMPONENT_TAR_NAME: &str = "slipwayhq.env.0.0.1.tar";
pub const SLIPWAY_ENV_JS_COMPONENT_TAR_NAME: &str = "slipwayhq.env_js.0.0.1.tar";

pub const SLIPWAY_CONTEXT_COMPONENT_NAME: &str = "slipwayhq.context.0.0.1";
pub const SLIPWAY_CONTEXT_COMPONENT_TAR_NAME: &str = "slipwayhq.context.0.0.1.tar";
pub const SLIPWAY_CONTEXT_JS_COMPONENT_TAR_NAME: &str = "slipwayhq.context_js.0.0.1.tar";

pub const SLIPWAY_INCREMENT_JSON_SCHEMA_COMPONENT_NAME: &str =
    "slipwayhq.increment_json_schema.0.0.1";
pub const SLIPWAY_INCREMENT_JSON_SCHEMA_COMPONENT_FOLDER_NAME: &str =
    "slipwayhq.increment_json_schema";
pub const SLIPWAY_INCREMENT_JSON_SCHEMA_COMPONENT_TAR_NAME: &str =
    "slipwayhq.increment_json_schema.0.0.1.tar";

pub const SLIPWAY_INCREMENT_TEN_COMPONENT_NAME: &str = "slipwayhq.increment_ten.0.0.1";
pub const SLIPWAY_INCREMENT_TEN_COMPONENT_TAR_NAME: &str = "slipwayhq.increment_ten.0.0.1.tar";

pub const SLIPWAY_FRAGMENT_COMPONENT_NAME: &str = "slipwayhq.fragment.0.0.1";
pub const SLIPWAY_FRAGMENT_COMPONENT_TAR_NAME: &str = "slipwayhq.fragment.0.0.1.tar";

pub const SLIPWAY_RIGGING_JS_COMPONENT_NAME: &str = "slipwayhq.rigging_js.0.0.1";
pub const SLIPWAY_RIGGING_JS_COMPONENT_TAR_NAME: &str = "slipwayhq.rigging_js.0.0.1.tar";

pub fn get_slipway_test_components_path() -> PathBuf {
    find_ancestor_path(PathBuf::from(SLIPWAY_TEST_COMPONENTS_PATH))
}

pub fn get_slipway_test_components_registry_url() -> String {
    let components_path = get_slipway_test_components_path();
    format!(
        "file://{}",
        components_path
            .join("{publisher}.{name}.{version}.tar")
            .to_string_lossy()
    )
}

pub fn find_ancestor_path(path_to_find: PathBuf) -> PathBuf {
    let mut current_path = std::env::current_dir().unwrap();

    let mut searched = Vec::new();
    loop {
        let current_search_path = current_path.join(&path_to_find);
        searched.push(current_search_path.clone());

        if current_search_path.exists() {
            return current_search_path;
        }

        if !current_path.pop() {
            panic!(
                "Could not find ancestor path: {path_to_find:#?}.\nSearched:\n{searched}\n",
                searched = searched
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<String>>()
                    .join("\n")
            );
        }
    }
}

pub fn find_files_with_extension(dir: &Path, extension: &str) -> Vec<String> {
    use walkdir::WalkDir;

    let mut files = Vec::new();
    for entry in WalkDir::new(dir).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == extension {
                    files.push(path.to_string_lossy().into_owned());
                }
            }
        }
    }
    files
}

pub fn quote(s: &str) -> String {
    format!(r#""{}""#, s)
}
