use std::env;

/// Created to define and use the environment variables in use by Firetrap.
pub enum Arg {
    NoDefault(&'static str),
    WithDefault(&'static str, &'static str),
}

impl Arg {
    pub fn name(&self) -> String {
        match self {
            Arg::NoDefault(name) | Arg::WithDefault(name, _) => name.to_string(),
        }
    }

    pub fn val(&self) -> String {
        match self {
            Arg::NoDefault(name) => env::var(name).unwrap(),
            Arg::WithDefault(name, default) => env::var(name).unwrap_or_else(|_| default.to_string()),
        }
    }

    pub fn provided(&self) -> bool {
        env::var_os(self.name()).is_some()
    }

    #[inline]
    pub fn val_or_else<F: FnOnce(std::env::VarError) -> String>(self, f: F) -> String {
        match self {
            Arg::NoDefault(name) => env::var(name).unwrap_or_else(f),
            Arg::WithDefault(name, default) => env::var(name).unwrap_or_else(|_| default.to_string()),
        }
    }
}
