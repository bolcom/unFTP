use std::env;
use std::process::Command;

fn main() {
    let version = match Command::new("git").args(["describe", "--tags"]).output() {
        Ok(output) => String::from_utf8(output.stdout).unwrap(),
        Err(_) => env::var("BUILD_VERSION").unwrap_or_else(|_| ">unknown<".to_string()),
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
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let src = std::path::Path::new(&manifest_dir);
    let dst = std::path::Path::new(&env::var("OUT_DIR").unwrap()).join("build-info.rs");
    let mut opts = built::Options::default();
    opts.set_dependencies(true);
    opts.set_git(true);
    opts.set_time(true);
    built::write_built_file_with_opts(&opts, src, &dst)
        .expect("Failed to acquire build-time information");
}
