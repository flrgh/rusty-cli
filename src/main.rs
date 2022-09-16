mod cli;
mod lua;
mod nginx;
mod run;
mod types;
mod util;

use crate::cli::*;
use crate::lua::*;
use crate::nginx::*;
use crate::run::run;
use crate::types::*;

use std::env;
use std::io::Write as IoWrite;
//use clap::{Parser, crate_version, CommandFactory};
use clap::*;
use std::process::{exit, Command};

fn main() {
    let app = letsgo();

    if let Err(e) = app {
        e.exit()
    }

    let app = app.unwrap();

    let nginx = &app.nginx;

    if app.version {
        eprintln!("rusty {}", crate_version!());
        let mut c = Command::new(nginx);
        c.arg("-V");
        exit(run(c))
    }

    let prefix = Prefix::new().expect("Failed creating prefix directory");

    let vars = Vars {
        main_conf: app.main_conf.clone(),
        stream_enabled: !app.no_stream,
        stream_conf: app.stream_conf.clone(),
        http_conf: app.http_conf.clone(),
        lua_loader: generate_lua_loader(&prefix, &app.lua_file, &app.inline_lua, &app.lua_args),
        worker_connections: app.worker_connections,
    };

    let conf_path = prefix.conf.join("nginx.conf");
    let mut fh = std::fs::File::create(&conf_path).unwrap();
    fh.write_all(render_config(vars).as_bytes()).unwrap();
    fh.flush().unwrap();
    drop(fh);

    let mut c = Command::new(nginx);
    c.args(["-p", prefix.root.to_str().unwrap(), "-c", "conf/nginx.conf"]);

    exit(run(c))
}
