use std::env;
use std::process::Command;

fn main() {
    let version = match env::var("BUILD_VERSION") {
        Ok(v) => v,
        _ => Command::new("git")
            .args(["describe", "--tags"])
            .output()
            .map(|o| String::from_utf8(o.stdout).unwrap())
            .unwrap_or_else(|_| ">unknown<".to_string()),
    };
    println!("cargo:rustc-env=BUILD_VERSION={}", version);

    println!(
        "cargo:rustc-env=PROJ_WEB_DIR={}/web",
        std::env::var("CARGO_MANIFEST_DIR").unwrap(),
    );

    // Didn't quite get that to work yet.
    // #[cfg(feature = "static")]
    // {
    //     println!("cargo:rustc-link-lib=static=pam");
    // }

    generate_build_info();
}

// uses the 'built' crate to generate a build.rs file with a bunch of build information. We then
// include this file in the app module.
fn generate_build_info() {
    built::write_built_file().expect("Failed to acquire build-time information");
}
