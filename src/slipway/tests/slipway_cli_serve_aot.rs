use assert_cmd::Command;
use common_macros::slipway_test_async;
use common_test_utils::get_slipway_test_components_path;
use std::process::{Child, Stdio};
use std::{thread, time::Duration};
use tempfile::tempdir;

mod common;
use common::*;

/// This test is similar to the quick-start tutorial.
#[slipway_test_async]
async fn slipway_cli_serve_aot_and_check_response() {
    // Create a temp dir for the server configuration.
    let dir = tempdir().unwrap();
    let path = dir.path();

    // Get the path to the slipway binary.
    let slipway_cmd = Command::cargo_bin("slipway").unwrap();
    let slipway_path = slipway_cmd.get_program();

    // Initialize the slipway server folder.
    Command::cargo_bin("slipway")
        .unwrap()
        .arg("serve")
        .arg(path)
        .arg("init")
        .assert()
        .success();

    // Create a rig.
    let rig_path = path.join("rigs");
    std::fs::write(
        rig_path.join("hello.json"),
        indoc::indoc! {r#"
        {
            "rigging": {
                "output": {
                    "component": "slipwayhq.increment.0.0.1",
                    "input": {
                        "type": "increment",
                        "value": 1
                    }
                }
            }
        }"#},
    )
    .unwrap();

    // Update the permissions so the rig can load the component.
    let mut config_json: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(path.join("slipway_serve.json")).unwrap())
            .unwrap();

    config_json["log_level"] = serde_json::json!("debug");

    config_json["rig_permissions"] = serde_json::json!({
            "hello": {
                "allow": [ { "permission": "all" } ]
            }
    });

    let components_path = get_slipway_test_components_path();
    let components_path_string = components_path.to_string_lossy();

    // Add components directory to registry URLs.
    config_json["registry_urls"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!(format!(
            "file:{components_path_string}/{{publisher}}.{{name}}.{{version}}.tar"
        )));

    std::fs::write(
        path.join("slipway_serve.json"),
        serde_json::to_string_pretty(&config_json).unwrap(),
    )
    .unwrap();

    // AOT compile WASM component.
    Command::cargo_bin("slipway")
        .unwrap()
        .env("RUST_BACKTRACE", "1")
        .arg("serve")
        .arg(path)
        .arg("aot-compile")
        .assert()
        .success();

    // Sanity check the directory structure.
    print_dir_structure(path, 2).unwrap();

    // Check there is now one file in the `aot` directory.
    let aot_path = path.join("aot");
    let entries = std::fs::read_dir(&aot_path).unwrap();
    let mut count = 0;
    for _ in entries {
        count += 1;
    }
    assert_eq!(count, 1);

    // Spawn the server as a child process
    let child: Child = std::process::Command::new(slipway_path)
        .arg("serve")
        .arg("--aot")
        .arg(path)
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to start slipway server");

    // Wait a moment for it to start
    thread::sleep(Duration::from_secs(1));

    // Make a request to check the server's response
    let maybe_response = reqwest::get("http://localhost:8080/rigs/hello?format=json").await;

    // Shut down the server
    send_ctrlc(&child); // child.kill().unwrap();

    let output = child.wait_with_output().unwrap();

    let response = maybe_response.unwrap();

    println!("{:?}", response);
    assert_eq!(response.status(), 200);

    let body = response.text().await.unwrap();
    println!("{:?}", body);
    let body_json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body_json["value"], 2);

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("{}", stdout);

    assert!(
        stdout.contains("Using AOT compiled WASM component"),
        "Failed to find string \"Using AOT compiled WASM component\" in stdout."
    );
}
