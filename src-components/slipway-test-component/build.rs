use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    // Get the path to the output directory
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_dir_path = PathBuf::from(out_dir);

    // Determine the target directory
    let target_dir = out_dir_path
        .ancestors()
        .nth(3)
        .expect("Failed to find target directory");

    // Define the source file and the destination file
    let source_file = "slipway_component.json";
    let dest_file = Path::new(target_dir).join(source_file);

    // Copy the file
    println!("Copying {} to {:?}", source_file, dest_file);
    fs::copy(source_file, dest_file).unwrap();

    // Re-run the build script if the source file changes
    println!("cargo:rerun-if-changed={}", source_file);
}
