use assert_cmd::Command;
use common_macros::slipway_test_async;
use std::process::Child;
use std::{thread, time::Duration};
use tempfile::tempdir;

mod common;
use common::*;

/// This test is similar to the quick-start tutorial.
#[slipway_test_async]
async fn slipway_cli_serve_and_check_response() {
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

    // Create a component.
    let component_path = path.join("components").join("demo.hello_world");
    std::fs::create_dir(&component_path).unwrap();
    std::fs::write(
        component_path.join("slipway_component.json"),
        indoc::indoc! {r#"
        {
            "publisher": "demo",
            "name": "hello_world",
            "version": "1.0.0",
            "input": {
                "properties": {
                "text": { "type": "string" }
                }
            },
            "output": {}
        }"#},
    )
    .unwrap();

    std::fs::write(
        component_path.join("run.js"),
        indoc::indoc! {r#"
            function run(input) {
                return {
                    "type": "AdaptiveCard",
                    "verticalContentAlignment": "center",
                    "body": [
                        {
                            "type": "TextBlock",
                            "horizontalAlignment": "center",
                            "text": input.text
                        }
                    ]
                };
            }

            export let output = run(input);
        "#},
    )
    .unwrap();

    // Create a rig.
    let rig_path = path.join("rigs");
    std::fs::write(
        rig_path.join("hello.json"),
        indoc::indoc! {r#"
        {
            "rigging": {
                "output": {
                    "component": "demo.hello_world.1.0.0",
                    "input": {
                        "text": "Hello World!"
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
    config_json["rig_permissions"] = serde_json::json!({
            "hello": {
                "allow": [ { "permission": "all" } ]
            }
    });
    std::fs::write(
        path.join("slipway_serve.json"),
        serde_json::to_string_pretty(&config_json).unwrap(),
    )
    .unwrap();

    // Sanity check the directory structure.
    print_dir_structure(path, 2).unwrap();

    // Spawn the server as a child process
    let mut child: Child = std::process::Command::new(slipway_path)
        .arg("serve")
        .arg(path)
        .spawn()
        .expect("Failed to start slipway server");

    // Wait a moment for it to start
    thread::sleep(Duration::from_secs(1));

    // Make a request to check the server's response
    let maybe_response = reqwest::get("http://localhost:8080/rigs/hello?format=json").await;

    // Shut down the server
    send_ctrlc(&child); // child.kill().unwrap();
    child.wait().unwrap();

    let response = maybe_response.unwrap();

    println!("{:?}", response);
    assert_eq!(response.status(), 200);

    let body = response.text().await.unwrap();
    println!("{:?}", body);
    assert!(body.contains("\"AdaptiveCard\""));
}
