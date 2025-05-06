use assert_cmd::Command;
use common_macros::slipway_test_async;
use common_test_utils::get_slipway_test_components_path;
use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderValue};
use slipway_engine::TEST_TIMEZONE;
use slipway_host::hash_string;
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
            export function run(input) {
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
    config_json["hashed_api_keys"] = serde_json::json!({
        "test": hash_string("test_api_key")
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
    let maybe_response =
        reqwest::get("http://localhost:8080/rigs/hello?format=json&authorization=test_api_key")
            .await;

    // Shut down the server
    send_ctrlc(&child); // child.kill().unwrap();
    child.wait().unwrap();

    let response = maybe_response.unwrap();
    let status_code = response.status();
    println!("{:?}", response);

    let body = response.text().await.unwrap();
    println!("{:?}", body);

    assert_eq!(status_code, 200);

    println!("{:?}", body);
    assert!(body.contains("\"AdaptiveCard\""));
}

/// This test checks the device context and timezone data.
#[slipway_test_async]
async fn slipway_cli_serve_device_and_check_context() {
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
                    "component": "slipwayhq.context.0.0.1",
                    "input": {
                        "context": "$.context"
                    }
                }
            }
        }"#},
    )
    .unwrap();

    // Create a playlist.
    let playlist_path = path.join("playlists");
    std::fs::write(
        playlist_path.join("hello_playlist.json"),
        indoc::indoc! {r#"
        {
            "schedule": [
                {
                    "refresh": {
                        "hours": 1
                    },
                    "rig": "hello"
                }
            ]
        }"#},
    )
    .unwrap();

    // Create a device.
    let device_path = path.join("devices");
    std::fs::write(
        device_path.join("hello_device.json"),
        indoc::indoc! {r#"
        {
            "playlist": "hello_playlist",
            "context": {
                "foo": "bar"
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
    config_json["hashed_api_keys"] = serde_json::json!({
        "test": hash_string("test_api_key")
    });
    config_json["timezone"] = serde_json::Value::String(TEST_TIMEZONE.to_string());

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
    let maybe_response = reqwest::get(
        "http://localhost:8080/devices/hello_device?format=json&authorization=test_api_key",
    )
    .await;

    // Shut down the server
    send_ctrlc(&child); // child.kill().unwrap();
    child.wait().unwrap();

    let response = maybe_response.unwrap();
    let status_code = response.status();
    println!("{:?}", response);

    let body = response.text().await.unwrap();
    println!("{:?}", body);

    assert_eq!(status_code, 200);

    println!("{:?}", body);
    let body_json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        body_json,
        serde_json::json!({
            "tz": TEST_TIMEZONE,
            "input": {
                "context": {
                    "timezone": TEST_TIMEZONE,
                    "device": {
                        "foo": "bar",
                    }
                }
            }
        })
    );
}

/// This test checks the TRMNL display API is working.
#[slipway_test_async]
async fn slipway_cli_serve_trmnl() {
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
                    "component": "passthrough",
                    "input": {
                        "canvas": {
                            "width": 20,
                            "height": 20,
                            "data": "////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////gYKB//////////////////////////////////////////////////////////////////////////////////////////////////////+BgoH//////////////////////////////////////////////////////////////////////////////////////////////////////4GCgf//////////////////////////////////////////////////////////////////////////////////////////////////////hYWF//////////////////////////////////////////////////////////////////////////////////////////////////////+TlJP//////////////////////////////////////////////////////////////////////////////////////////////////////6Oko///////////////////////////////////////////////////////////////////////////////////////////////////////tbW1///////////////////////////////////////////////////////////////////////////////////////////////////////FxcX/////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////iImI/////////////////w=="
                        }
                    }
                }
            }
        }"#},
    )
    .unwrap();

    // Create a playlist.
    let playlist_path = path.join("playlists");
    std::fs::write(
        playlist_path.join("hello_playlist.json"),
        indoc::indoc! {r#"
        {
            "schedule": [
                {
                    "refresh": {
                        "hours": 1
                    },
                    "rig": "hello"
                }
            ]
        }"#},
    )
    .unwrap();

    // Create a device.
    let device_path = path.join("devices");
    std::fs::write(
        device_path.join("hello_device.json"),
        serde_json::json!(
        {
            "playlist": "hello_playlist",
            "trmnl": {
                "id": "my_trmnl_id",
                "hashed_api_key": hash_string("trmnl_test_api_key"),
                "reset_firmware": false
            },
            "context": {
                "foo": "bar"
            }
        })
        .to_string(),
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
    config_json["hashed_api_keys"] = serde_json::json!({
        "test": hash_string("test_api_key")
    });
    config_json["timezone"] = serde_json::Value::String(TEST_TIMEZONE.to_string());

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

    // Sanity check the directory structure.
    print_dir_structure(path, 2).unwrap();

    // Spawn the server as a child process
    let mut child: Child = std::process::Command::new(slipway_path)
        .arg("serve")
        .arg(path)
        .env("SLIPWAY_SECRET", "test_slipway_secret")
        .spawn()
        .expect("Failed to start slipway server");

    // Wait a moment for it to start
    thread::sleep(Duration::from_secs(1));

    // Make a request to check the server's response
    let client = Client::new();
    let mut headers = HeaderMap::new();
    headers.insert("id", HeaderValue::from_static("my_trmnl_id"));
    headers.insert(
        "access-token",
        HeaderValue::from_static("trmnl_test_api_key"),
    );
    let maybe_response = client
        .get("http://localhost:8080/trmnl/api/display")
        .headers(headers)
        .send()
        .await;

    let response = maybe_response.unwrap();
    let status_code = response.status();
    println!("{:?}", response);

    assert_eq!(status_code, 200);

    let body = response.json::<serde_json::Value>().await.unwrap();
    println!("{:?}", body);

    let image_url = body["image_url"].as_str().unwrap();

    let maybe_response = client.get(image_url).send().await;

    // Shut down the server
    send_ctrlc(&child); // child.kill().unwrap();
    child.wait().unwrap();

    let response = maybe_response.unwrap();
    println!("{:?}", response);

    let status_code = response.status();
    let content_type = response.headers().get("content-type").unwrap();

    assert_eq!(status_code, 200);
    assert_eq!(content_type, "image/bmp");

    let body = response.bytes().await.unwrap();
    println!("Body length: {}", body.len());
    assert!(body.len() > (20 * 20) / 8);
}
