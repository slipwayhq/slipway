use assert_cmd::Command;
use std::{
    path::Path,
    process::{Child, Stdio},
    thread,
    time::Duration,
};

pub fn send_ctrlc(child: &Child) {
    use nix::sys::signal::{Signal, kill};
    use nix::unistd::Pid;
    kill(Pid::from_raw(child.id() as i32), Signal::SIGINT)
        .expect("Failed to send SIGINT to child process");
}

pub fn print_dir_structure(path: &Path, indent: usize) -> std::io::Result<()> {
    if path.is_dir() {
        let entries = std::fs::read_dir(path)?;
        for entry in entries {
            let entry = entry?;
            let file_type = entry.file_type()?;
            for _ in 0..indent {
                print!("  ");
            }
            println!("{}", entry.file_name().to_string_lossy());

            if file_type.is_dir() {
                print_dir_structure(&entry.path(), indent + 1)?;
            }
        }
    }
    Ok(())
}

// Create a guard to ensure the server is shut down at the end.
pub struct ServerGuard {
    child: Option<Child>,
}

impl ServerGuard {
    #[allow(dead_code)]
    pub fn new(path: &Path, aot: bool) -> Self {
        // Get the path to the slipway binary.
        let slipway_cmd = Command::cargo_bin("slipway").unwrap();
        let slipway_path = slipway_cmd.get_program();

        let mut command = std::process::Command::new(slipway_path);

        command.arg("serve").arg(path).stdout(Stdio::piped());

        if aot {
            command.arg("--aot");
        }

        let child = command.spawn().expect("Failed to start slipway server");

        // Wait a moment for it to start
        thread::sleep(Duration::from_secs(1));

        ServerGuard { child: Some(child) }
    }

    #[allow(dead_code)]
    pub fn kill_and_get_output(&mut self) -> Option<std::process::Output> {
        if let Some(child) = self.child.take() {
            send_ctrlc(&child); // Send shutdown signal
            let output = child.wait_with_output().unwrap();
            Some(output)
        } else {
            None
        }
    }
}

impl Drop for ServerGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            send_ctrlc(&child); // Send shutdown signal
            let _ = child.wait();
        }
    }
}
