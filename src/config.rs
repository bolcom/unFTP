use std::env;

/// Created to define and use the environment variables in use by Firetrap.
pub enum EnvVar {
    NoDefault(&'static str),
    WithDefault(&'static str, &'static str),
}

impl EnvVar {
    pub fn name(&self) -> String {
        match self {
            EnvVar::NoDefault(name) | EnvVar::WithDefault(name, _) => name.to_string(),
        }
    }

    pub fn val(&self) -> String {
        match self {
            EnvVar::NoDefault(name) => env::var(name).unwrap(),
            EnvVar::WithDefault(name, default) => env::var(name).unwrap_or(default.to_string()),
        }
    }

    pub fn provided(&self) -> bool {
        match env::var_os(self.name()) {
            Some(_) => true,
            None => false,
        }
    }
}
