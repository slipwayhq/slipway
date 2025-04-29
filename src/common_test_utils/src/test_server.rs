use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::thread;
use tiny_http::Method;
use tiny_http::Response;
use tiny_http::Server;

static LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

const LOCALHOST_BINDING: &str = "127.0.0.1:0";

// Simple test server used for testing things like schema HTTP resolution
// is working as expected.
pub struct TestServer {
    #[allow(dead_code)]
    mutex: MutexGuard<'static, ()>,
    stop_signal: Sender<char>,
    server_thread: Option<thread::JoinHandle<()>>,
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
        let mutex = LOCK.lock().unwrap();

        let (tx, rx) = mpsc::channel();

        let server = Server::http(LOCALHOST_BINDING).unwrap();
        let localhost_url = get_localhost_url(&server);

        let server_thread = thread::spawn(move || {
            loop {
                // Check for stop signal in a non-blocking way
                if rx.try_recv().is_ok() {
                    break;
                }

                // Handle incoming requests
                if let Ok(Some(request)) =
                    server.recv_timeout(std::time::Duration::from_millis(100))
                {
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
            }
        });

        TestServer {
            mutex,
            stop_signal: tx,
            server_thread: Some(server_thread),
            localhost_url,
        }
    }

    pub fn start_from_folder(folder: PathBuf) -> Self {
        let mutex = LOCK.lock().unwrap();
        let folder = crate::find_ancestor_path(folder.clone());

        let (tx, rx) = mpsc::channel();

        let server = Server::http(LOCALHOST_BINDING).unwrap();
        let localhost_url = get_localhost_url(&server);
        let server_thread = thread::spawn(move || {
            loop {
                // Check for stop signal in a non-blocking way
                if rx.try_recv().is_ok() {
                    break;
                }

                // Handle incoming requests
                if let Ok(Some(request)) =
                    server.recv_timeout(std::time::Duration::from_millis(100))
                {
                    let file = folder.join(request.url().trim_start_matches('/'));

                    if !file.exists() {
                        println!("Not found URL: {}", request.url());
                        println!("Using file: {:?}", file);
                        request
                            .respond(Response::from_string("Not found").with_status_code(404))
                            .unwrap();
                        continue;
                    }

                    // Stream file as response
                    let bytes = std::fs::read(&file).unwrap();

                    let response = Response::from_data(bytes).with_chunked_threshold(usize::MAX);
                    request.respond(response).unwrap();
                }
            }
        });

        TestServer {
            mutex,
            stop_signal: tx,
            server_thread: Some(server_thread),
            localhost_url,
        }
    }

    pub fn start_for_call(
        url: String,
        method: String,
        headers: Vec<(String, String)>,
        body: String,
        status_code: u16,
    ) -> Self {
        let mutex = LOCK.lock().unwrap();

        let (tx, rx) = mpsc::channel();

        let server = Server::http(LOCALHOST_BINDING).unwrap();
        let localhost_url = get_localhost_url(&server);

        let server_thread = thread::spawn(move || {
            loop {
                // Check for stop signal in a non-blocking way
                if rx.try_recv().is_ok() {
                    break;
                }

                // Handle incoming requests
                if let Ok(Some(request)) =
                    server.recv_timeout(std::time::Duration::from_millis(100))
                {
                    if url != request.url() {
                        panic!("Unexpected url: {}", request.url());
                    }

                    if request.method() != &Method::from_str(&method).unwrap() {
                        panic!("Unexpected method: {:?}", request.method());
                    }

                    if request.body_length().unwrap_or(0) != body.len() {
                        panic!(
                            "Unexpected body length: {}",
                            request.body_length().unwrap_or(0)
                        );
                    }

                    let actual_headers = request.headers();
                    for (key, value) in headers.iter() {
                        let actual_header = actual_headers
                            .iter()
                            .find(|h| h.field.as_str() == key.as_str());

                        if let Some(actual_header) = actual_header {
                            assert_eq!(actual_header.value, *value);
                        } else {
                            panic!("Expected header not found: {}", key);
                        }
                    }
                    let response =
                        Response::from_string(body.clone()).with_status_code(status_code);
                    request.respond(response).unwrap();
                }
            }
        });

        TestServer {
            mutex,
            stop_signal: tx,
            server_thread: Some(server_thread),
            localhost_url,
        }
    }

    pub fn stop(mut self) {
        self.stop_signal.send('a').unwrap();
        match self.server_thread.take() {
            Some(h) => h.join(),
            None => Ok(()),
        }
        .unwrap()
    }
}

// impl Drop for TestServer {
//     fn drop(&mut self) {
//         if self.server_thread.is_some() {
//             panic!("TestServer was not stopped before being dropped");
//         }
//     }
// }
