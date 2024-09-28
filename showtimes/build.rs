use std::process::Command;

fn main() {
    let process = Command::new("git").args(["rev-parse", "HEAD"]).output();

    match process {
        Ok(output) => {
            let commit = String::from_utf8_lossy(&output.stdout);

            println!("cargo:rustc-env=GIT_COMMIT={}", commit);
        }
        Err(_) => {
            println!("cargo:rustc-env=GIT_COMMIT=unknown");
        }
    }
}
