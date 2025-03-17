use std::{path::Path, process::Child};

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
