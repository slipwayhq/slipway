use std::path::PathBuf;

pub(crate) const SLIPWAY_TEST_COMPONENT_PATH: &str = "./test-components/slipway_test_component";

pub(crate) fn find_ancestor_path(path_to_find: PathBuf) -> PathBuf {
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
