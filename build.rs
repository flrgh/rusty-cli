use std::env;

fn main() {
    println!("cargo::rerun-if-env-changed=NGINX_PATH");
    if env::var("NGINX_PATH").is_ok_and(|nginx| !nginx.trim().is_empty()) {
        println!("cargo::rustc-cfg=default_nginx_path");
    }
}
