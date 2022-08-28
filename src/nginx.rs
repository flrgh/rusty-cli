use minijinja::{context, Environment};
use std::env;
use std::path::PathBuf;

pub static TEMPLATE: &str = include_str!("nginx.conf.tpl");

pub struct Vars {
    pub main_conf: Vec<String>,
    pub stream_enabled: bool,
    pub stream_conf: Vec<String>,
    pub http_conf: Vec<String>,
    pub lua_loader: Vec<String>,
    pub worker_connections: u32,
}

pub fn render_config(vars: Vars) -> String {
    let mut env = Environment::new();
    env.add_template("nginx.conf", TEMPLATE).unwrap();
    let template = env.get_template("nginx.conf").unwrap();

    let ctx = context! {
        main_conf => vars.main_conf,
        http_conf => vars.http_conf,
        stream_enabled => vars.stream_enabled,
        stream_conf => vars.stream_conf,
        lua_loader => vars.lua_loader,
        worker_connections => vars.worker_connections,
    };

    template.render(ctx).unwrap()
}

pub fn find_nginx_bin(nginx: Option<String>) -> PathBuf {
    if let Some(path) = nginx {
        return PathBuf::from(path);
    }

    let bin = env::current_exe().unwrap();
    let parent = match bin.parent() {
        Some(p) => p.to_path_buf(),
        None => PathBuf::from("/"),
    };

    let nginx = parent.join("nginx/sbin/nginx");
    if nginx.is_file() {
        return nginx;
    }

    let nginx = parent.join("nginx");
    if nginx.is_file() {
        return nginx;
    }

    PathBuf::from("nginx")
}
