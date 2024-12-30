use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::thread;
use tiny_http::Response;
use tiny_http::Server;

const LOCALHOST_BINDING: &str = "0.0.0.0:0";

// Simple test server used for testing things like schema HTTP resolution
// is working as expected.
pub struct TestServer {
    stop_signal: Sender<char>,
    server_thread: thread::JoinHandle<()>,
    pub localhost_url: String,
}

fn get_localhost_url(server: &Server) -> String {
    let url =
        format!("http://{}", server.server_addr().to_ip().unwrap()).replace("0.0.0.0", "localhost");

    // Ensure url ends in /
    let url = if url.ends_with('/') {
        url
    } else {
        format!("{}/", url)
    };

    println!("Test server started at: {}", url);
    url
}

impl TestServer {
    pub fn start_from_string_map(responses: HashMap<String, String>) -> Self {
        let (tx, rx) = mpsc::channel();

        let server = Server::http(LOCALHOST_BINDING).unwrap();
        let localhost_url = get_localhost_url(&server);

        let server_thread = thread::spawn(move || loop {
            // Check for stop signal in a non-blocking way
            if rx.try_recv().is_ok() {
                break;
            }

            // Handle incoming requests
            if let Ok(Some(request)) = server.recv_timeout(std::time::Duration::from_millis(100)) {
                match responses.get(request.url()) {
                    None => {
                        println!("Not found: {}", request.url());
                        request
                            .respond(Response::from_string("Not found").with_status_code(404))
                            .unwrap();
                        continue;
                    }
                    Some(response_str) => {
                        request
                            .respond(Response::from_string(response_str))
                            .unwrap();
                    }
                }
            }
        });

        TestServer {
            stop_signal: tx,
            server_thread,
            localhost_url,
        }
    }

    pub fn start_from_folder(folder: PathBuf) -> Self {
        let (tx, rx) = mpsc::channel();

        let server = Server::http(LOCALHOST_BINDING).unwrap();
        let localhost_url = get_localhost_url(&server);
        let server_thread = thread::spawn(move || loop {
            // Check for stop signal in a non-blocking way
            if rx.try_recv().is_ok() {
                break;
            }

            // Handle incoming requests
            if let Ok(Some(request)) = server.recv_timeout(std::time::Duration::from_millis(100)) {
                let folder = crate::find_ancestor_path(folder.clone());
                let file = folder.join(request.url().trim_start_matches('/'));

                if !file.exists() {
                    println!("Not found: {}", request.url());
                    request
                        .respond(Response::from_string("Not found").with_status_code(404))
                        .unwrap();
                    continue;
                }

                // Stream file as response
                let file = std::fs::read(file).unwrap();
                let response = Response::from_data(file);
                request.respond(response).unwrap();
            }
        });

        TestServer {
            stop_signal: tx,
            server_thread,
            localhost_url,
        }
    }

    pub fn stop(self) {
        self.stop_signal.send('a').unwrap();
        self.server_thread.join().unwrap();
    }
}
