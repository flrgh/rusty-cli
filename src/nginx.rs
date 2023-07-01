use minijinja::{context, Environment};
use std::env;
use std::path::PathBuf;

const RESTY_COMPAT_VAR: &str = "RESTY_CLI_COMPAT_VERSION";
const RESTY_COMPAT_LATEST: u64 = 28;

const TEMPLATE: &str = include_str!("nginx.conf.tpl");
const TEMPLATE_NAME: &str = "nginx.conf";

pub struct Vars {
    pub events_conf: Vec<String>,
    pub main_conf: Vec<String>,
    pub stream_enabled: bool,
    pub stream_conf: Vec<String>,
    pub http_conf: Vec<String>,
    pub lua_loader: Vec<String>,
}

fn init_template<'a>(env: &'a mut Environment) -> minijinja::Template<'a> {
    env.add_template(TEMPLATE_NAME, TEMPLATE).unwrap();
    env.get_template(TEMPLATE_NAME).unwrap()
}

#[test]
fn verify_template() {
    let mut env = Environment::new();
    init_template(&mut env);
}

pub fn render_config(vars: Vars) -> String {
    let mut env = Environment::new();
    let template = init_template(&mut env);

    let ctx = context! {
        main_conf => vars.main_conf,
        http_conf => vars.http_conf,
        stream_enabled => vars.stream_enabled,
        stream_conf => vars.stream_conf,
        lua_loader => vars.lua_loader,
        events_conf => vars.events_conf,
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
    // TODO: maybe make this a build config item?
    match env::var_os(RESTY_COMPAT_VAR) {
        Some(value) => {
            let value = value.to_str().unwrap();

            let value = value.strip_prefix('v').unwrap_or(value);

            let items: Vec<&str> = value.splitn(3, '.').collect();

            let value = if items.len() > 1 { items[1] } else { items[0] };

            value.parse::<u64>().unwrap_or(RESTY_COMPAT_LATEST)
        }
        None => RESTY_COMPAT_LATEST,
    }
}
