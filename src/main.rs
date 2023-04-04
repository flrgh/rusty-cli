#[macro_use]
extern crate lazy_static;

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

use clap::*;
use std::env;
use std::error::Error;
use std::io::Write as IoWrite;
use std::process::{exit, Command};

fn main() {
    let app = letsgo();

    if let Err(e) = app {
        use clap::error::ErrorKind::*;

        if e.kind() == DisplayHelp {
            e.exit()
        }

        // uggggggghhhhhh
        //
        // resty-cli uses `die` indiscriminantly, which makes it a PITA to
        // determine what exit code to use

        let ec = match e.kind() {
            InvalidValue => 255,
            WrongNumberOfValues => 255,
            TooManyValues => 255,
            TooFewValues => 255,
            MissingRequiredArgument => 255,
            ValueValidation => 255,

            // yup, resty-cli returns 25 (ENOTTY) for mutually-exclusive
            // arguments
            //
            // not on purpose though, it's just a side effect of errno
            // having been set from a previous and unrelated error
            ArgumentConflict => 25,

            InvalidUtf8 => 255,
            Io => 2,

            UnknownArgument => 1,
            DisplayHelp => 0,

            DisplayHelpOnMissingArgumentOrSubcommand => unreachable!(),
            Format => unreachable!(),

            DisplayVersion => unreachable!(),
            MissingSubcommand => unreachable!(),
            InvalidSubcommand => unreachable!(),
            NoEquals => unreachable!(),
            _ => unreachable!(),
        };

        //dbg!(&e);

        // if let Some(src) = e.source() {
        //     eprintln!("{}", src);
        // } else {
        //     eprint!("{}", e.to_string());
        // }

        eprint!("{}", e.to_string());

        std::process::exit(ec);
    }

    let mut app = app.unwrap();

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
    let mut fh = std::fs::File::create(conf_path).unwrap();
    fh.write_all(render_config(vars).as_bytes()).unwrap();
    fh.flush().unwrap();
    drop(fh);

    app.prefix = Some(prefix.root.to_str().unwrap().to_owned());
    let ec = run(Command::from(app));

    drop(prefix);

    exit(ec)
}
