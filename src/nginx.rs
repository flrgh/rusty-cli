use minijinja::{context, Environment};
use std::env;
use std::path::PathBuf;

pub static RESTY_COMPAT_VAR: &str = "RESTY_CLI_COMPAT_VERSION";
pub static RESTY_COMPAT_LATEST: u64 = 29;

pub static TEMPLATE: &str = include_str!("nginx.conf.tpl");

pub struct Vars {
    pub main_conf: Vec<String>,
    pub stream_enabled: bool,
    pub stream_conf: Vec<String>,
    pub http_conf: Vec<String>,
    pub lua_loader: Vec<String>,
    pub worker_connections: u32,
}

#[test]
fn verify_template() {
    let mut env = Environment::new();
    env.add_template("nginx.conf", TEMPLATE).unwrap();
    env.get_template("nginx.conf").unwrap();
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
        resty_compat_version => get_resty_compat_version(),
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


fn get_resty_compat_version() -> u64 {
    match env::var_os(RESTY_COMPAT_VAR) {
        Some(value) => {
            let value = value.to_str().unwrap();

            let value = value.strip_prefix("v")
                            .unwrap_or(value);

            let items: Vec<&str> = value.splitn(3, ".").collect();

            let value = if items.len() > 1 {
                items[1]
            } else {
                items[0]
            };

            value.parse::<u64>().unwrap_or(RESTY_COMPAT_LATEST)
        },
        None => {
            RESTY_COMPAT_LATEST
        }
    }
}
