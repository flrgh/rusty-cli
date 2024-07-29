use std::env;

fn main() {
    println!("cargo::rustc-check-cfg=cfg(default_nginx_path)");
    println!("cargo::rerun-if-env-changed=NGINX_PATH");
    if env::var("NGINX_PATH").is_ok_and(|nginx| !nginx.trim().is_empty()) {
        println!("cargo::rustc-cfg=default_nginx_path");
    }
}
