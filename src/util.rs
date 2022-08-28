//use mktemp::Temp;
use std::fs;
use std::io::{BufRead, BufReader};
use std::net::IpAddr;

// pub fn render(prefix: &Prefix, args: RestyArgs) -> String {
//     use crate::nginx::{render_config, Vars};
//
//     let loader: Vec<String> =
//         generate_lua_loader(prefix.root.as_path(), &args.lua_file, &args.prog)
//             .lines()
//             .map(|l| l.to_string())
//             .collect();
//
//     render_config(Vars {
//         env: env_vars(),
//         log_level: args.errlog_level.to_string(),
//         main_conf: main_conf(&args),
//         stream_enabled: !args.no_stream,
//         stream_conf: stream_conf(&args),
//         http_conf: http_conf(&args),
//         lua_args: args.lua_args.clone(),
//         lua_loader: loader,
//         worker_connections: args.count,
//     })
// }

pub fn try_parse_resolv_conf() -> Option<Vec<String>> {
    let file: fs::File;

    if let Ok(fh) = fs::File::open("/etc/resolv.conf") {
        file = fh
    } else {
        return None;
    }

    let mut nameservers = vec![];

    BufReader::new(file)
        .lines()
        .take_while(Result::is_ok)
        .map(Result::unwrap)
        .for_each(|line| {
            let line = line.trim();
            let mut parts = line.split_whitespace();

            let predicate = match parts.next() {
                Some("nameserver") => parts.next(),
                _ => None,
            };

            // not enough parts
            if predicate.is_none() {
                return;
            }

            // too many parts
            if parts.next().is_some() {
                return;
            }

            let s = predicate.unwrap();

            if let Ok(addr) = s.parse::<IpAddr>() {
                nameservers.push(addr.to_string());
            }
        });

    Some(nameservers)
}
