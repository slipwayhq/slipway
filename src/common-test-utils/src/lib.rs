use std::path::{Path, PathBuf};

pub mod test_server;

pub const SLIPWAY_TEST_COMPONENTS_PATH: &str = "./test-components";
pub const SLIPWAY_TEST_COMPONENT_NAME: &str = "slipway_test_component";
pub const SLIPWAY_TEST_COMPONENT_JSON_SCHEMA_NAME: &str = "slipway_test_component_json_schema";
pub const SLIPWAY_TEST_COMPONENT_JSON_SCHEMA_TAR_NAME: &str =
    "slipway_test_component_json_schema.tar";

pub fn get_slipway_test_component_path(component_name: &str) -> PathBuf {
    find_ancestor_path(PathBuf::from(SLIPWAY_TEST_COMPONENTS_PATH).join(component_name))
}

pub fn get_slipway_test_components_path() -> PathBuf {
    find_ancestor_path(PathBuf::from(SLIPWAY_TEST_COMPONENTS_PATH))
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
                "Could not find ancestor path: {path_to_find:?}.\nSearched:\n{searched}\n",
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
