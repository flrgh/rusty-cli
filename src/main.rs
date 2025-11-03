use std::sync::LazyLock;

use crate::compat_version::Version;

mod cli;
mod compat_version;
mod lua;
mod nginx;
mod run;
mod types;
mod util;

pub(crate) const VERSION: &str = env!("CARGO_PKG_VERSION");
pub(crate) const RUSTY_CLI: &str = "rusty-cli";

#[allow(dead_code)]
pub(crate) const NAME: &str = match option_env!("CARGO_BIN_NAME") {
    Some(name) => name,
    None => RUSTY_CLI,
};

#[cfg(default_resty_compat_version)]
pub(crate) const RESTY_COMPAT_DEFAULT: compat_version::Version = {
    match compat_version::Version::from_str(env!("RESTY_CLI_COMPAT_VERSION")) {
        Some(version) => version,
        // we already validated the version in build.rs
        None => unreachable!(),
    }
};

#[cfg(not(default_resty_compat_version))]
pub(crate) const RESTY_COMPAT_DEFAULT: Version = compat_version::RESTY_COMPAT_MAX;

pub(crate) static RESTY_COMPAT_VERSION: LazyLock<Version> = LazyLock::new(|| {
    use compat_version::*;
    match Version::from_env() {
        Some(Ok(value)) => {
            if value > RESTY_COMPAT_MAX {
                eprintln!("WARN: {RESTY_COMPAT_VAR} ({value}) is greater than max supported version ({RESTY_COMPAT_MAX})");
            } else if value < RESTY_COMPAT_MIN {
                eprintln!("WARN: {RESTY_COMPAT_VAR} ({value}) is less than the minimum supported version ({RESTY_COMPAT_MIN})");
            }
            Some(value)
        },
        Some(Err(value)) => {
            eprintln!("WARN: value of {RESTY_COMPAT_VAR} env var (`{value}`) is invalid");
            None
        }
        None => None,
    }
        .unwrap_or(RESTY_COMPAT_DEFAULT)
});

fn main() {
    use std::process::exit;

    match cli::Action::try_from(std::env::args()) {
        Err(e) => {
            eprintln!("{}", e);
            exit(e.exit_code());
        }

        Ok(action) => {
            exit(action.run());
        }
    }
}
