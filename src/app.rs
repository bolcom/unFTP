#![allow(dead_code)]

/// The application name
pub const NAME: &str = "unFTP";

/// The application version
pub const VERSION: &str = env!("BUILD_VERSION");

// The file has been placed here by the build script. See build.rs
include!(concat!(env!("OUT_DIR"), "/built.rs"));

lazy_static! {
    static ref LONG_VERSION: String = {
        [
            PKG_VERSION.to_string(),
            format!(" - Git version:\t{}", GIT_VERSION.unwrap_or("unknown")),
            format!(" - Built:\t{}", BUILT_TIME_UTC),
            format!(" - libunftp:\tv{}", libunftp_version()),
            format!(" - Compiler:\t{}", RUSTC_VERSION),
            format!(" - OS/Arch:\t{}/{}", CFG_OS, CFG_TARGET_ARCH),
            format!(" - Features:\t{}", FEATURES_STR),
            format!(" - Debug:\t{}", DEBUG),
        ]
        .join("\n")
    };
}

pub fn long_version() -> &'static str {
    (*LONG_VERSION).as_str()
}

pub fn libunftp_version() -> &'static str {
    let libunftp_version = DEPENDENCIES.iter().find_map(|(name, version)| match *name {
        "libunftp" => Some(version),
        _ => None,
    });
    libunftp_version.unwrap_or(&"unknown")
}
