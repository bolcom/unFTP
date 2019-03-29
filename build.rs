use std::env;
use std::process::Command;

fn main() {
    let version = match Command::new("git").args(&["describe", "--tags"]).output() {
        Ok(output) => String::from_utf8(output.stdout).unwrap(),
        Err(_) => env::var("BUILD_VERSION").unwrap_or(">unknown<".to_string()),
    };
    println!("cargo:rustc-env=BUILD_VERSION={}", version);
}
