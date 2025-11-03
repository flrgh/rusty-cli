use std::env;

mod compat_version {
    include!("src/compat_version.rs");
}

fn main() {
    configure_nginx_path();
    configure_resty_compat();
}

fn configure_nginx_path() {
    println!("cargo::rustc-check-cfg=cfg(default_nginx_path)");
    println!("cargo::rerun-if-env-changed=NGINX_PATH");

    if env::var("NGINX_PATH").is_ok_and(|nginx| !nginx.trim().is_empty()) {
        println!("cargo::rustc-cfg=default_nginx_path");
    }
}

fn configure_resty_compat() {
    use compat_version::*;

    println!("cargo::rustc-check-cfg=cfg(default_resty_compat_version)");
    println!("cargo::rerun-if-env-changed={RESTY_COMPAT_VAR}");

    match Version::from_env() {
        Some(Ok(version)) => {
            println!("cargo::rustc-cfg=default_resty_compat_version=\"{version}\"");

            if version > RESTY_COMPAT_MAX {
                println!("cargo::warning={RESTY_COMPAT_VAR} ({version}) is greater than max supported version ({RESTY_COMPAT_MAX})");
            } else if version < RESTY_COMPAT_MIN {
                println!("cargo::warning={RESTY_COMPAT_VAR} ({version}) is less than the minimum supported version ({RESTY_COMPAT_MIN})");
            }
        }
        Some(Err(value)) => {
            println!("cargo::error=invalid {RESTY_COMPAT_VAR}: `{value}`");
        }
        None => {}
    }
}
