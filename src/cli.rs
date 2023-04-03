use crate::lua::*;
use crate::nginx::*;
use crate::types::*;
use crate::util::*;
use clap::error::ErrorKind;
use clap::*;
use std::collections::{HashMap, VecDeque};
use std::convert::{From, TryFrom};
use std::env;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::fs;
use std::path::PathBuf;
use std::process;

#[derive(Debug, Clone)]
struct MissingIncludeFileError {
    section: String,
    filename: String,
}

impl Display for MissingIncludeFileError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "could not find {} include file '{}'",
            self.section, self.filename
        )
    }
}

impl From<MissingIncludeFileError> for clap::error::Error {
    fn from(value: MissingIncludeFileError) -> Self {
        eprintln!("ERROR: {}", value);
        clap::Error::new(ErrorKind::Io)
    }
}

type IncludeResult = Result<(), MissingIncludeFileError>;

fn confs(field: &mut Vec<String>, id: &str, m: &mut ArgMatches) {
    for line in consume_arg_strings(id, m) {
        field.push(normalize_conf_line(line));
    }
}

fn includes(field: &mut Vec<String>, id: &str, m: &mut ArgMatches) -> IncludeResult {
    // main-conf => main
    let section = id.split_once('-').map_or(id, |t| t.0);

    for p in consume_arg_strings(id, m) {
        let path = fs::canonicalize(p.clone()).map_err(|_| MissingIncludeFileError {
            section: section.to_string(),
            filename: p.clone(),
        })?;

        let s = path.to_str().unwrap().to_string();

        if !path.is_file() {
            eprintln!("WTF {}", s);
            return Err(MissingIncludeFileError {
                section: section.to_string(),
                filename: p,
            });
        }
        field.push(format!("include {};", s));
    }
    Ok(())
}

fn resolver(app: &App) -> String {
    let mut ns = app.nameservers.clone();
    if !app.resolve_ipv6 {
        ns.push("ipv6=off".to_string());
    }

    format!("resolver {};", ns.join(" "))
}

fn env_vars() -> Vec<String> {
    let mut vars: Vec<String> = env::vars()
        .map(|(name, _)| format!("env {};", name))
        .collect();

    vars.sort();
    vars
}

fn normalize_conf_line(line: String) -> String {
    let line = line.trim();
    let line = line.trim_end_matches(';');
    format!("{};", line)
}

fn http_conf(app: &mut App, m: &mut ArgMatches) -> IncludeResult {
    app.http_conf.push(resolver(app));
    app.http_conf.extend(package_path(&app.lua_package_path));
    app.http_conf.extend(package_cpath(&app.lua_package_path));

    for shm in consume_arg_strings("shdict", m) {
        // TODO: validation
        app.http_conf.push(format!("lua_shared_dict {};", shm));
    }

    confs(&mut app.http_conf, "http-conf", m);
    includes(&mut app.http_conf, "http-include", m)
}

fn stream_conf(app: &mut App, m: &mut ArgMatches) {
    if app.no_stream {
        return;
    }
    app.stream_conf.push(resolver(app));
    app.stream_conf.extend(package_path(&app.lua_package_path));
    app.stream_conf.extend(package_cpath(&app.lua_package_path));
    extend_from_args(&mut app.stream_conf, "stream-conf", m);
}

fn main_conf(app: &mut App, m: &mut ArgMatches) -> IncludeResult {
    app.main_conf.extend(env_vars());
    app.main_conf
        .push(format!("error_log stderr {};", app.errlog_level.to_owned()));
    confs(&mut app.main_conf, "main-conf", m);
    includes(&mut app.main_conf, "main-include", m)
}

fn consume_arg_strings(id: &str, m: &mut ArgMatches) -> impl Iterator<Item = String> {
    m.remove_many::<String>(id).unwrap_or_default()
}

fn consume_arg_string(id: &str, m: &mut ArgMatches) -> Option<String> {
    m.remove_one::<String>(id)
}

fn consume_flag(id: &str, m: &mut ArgMatches) -> bool {
    m.remove_one::<bool>(id).unwrap_or_default()
}

fn extend_from_args(field: &mut Vec<String>, id: &str, m: &mut ArgMatches) {
    field.extend(consume_arg_strings(id, m));
}

fn arg_is_present(id: &str, m: &ArgMatches) -> bool {
    m.get_one::<String>(id).is_some()
}

fn set_from_args<T>(field: &mut T, id: &str, m: &mut ArgMatches)
where
    T: Default + Clone + Sync + Send + 'static,
{
    *field = m.remove_one::<T>(id).unwrap_or_default();
}

fn nameservers(app: &mut App, m: &mut ArgMatches) {
    // take user input first
    app.nameservers.extend(
        m.remove_many::<IpAddr>("nameservers")
            .unwrap_or_default()
            .map(|ip| ip.to_string()),
    );

    if !app.nameservers.is_empty() {
        return;
    }

    // try to parse /etc/resolv.conf next
    app.nameservers
        .extend(try_parse_resolv_conf().into_iter().flatten());

    if !app.nameservers.is_empty() {
        return;
    }

    // fall back to google dns for compatibility with resty-cli
    app.nameservers.push("8.8.8.8".to_owned());
    app.nameservers.push("8.8.4.4".to_owned());
}

fn cmd() -> Command {
    use builder::*;
    use ArgAction::*;

    Command::new("rusty-cli")
        .trailing_var_arg(true)
        .next_line_help(true)
        .dont_collapse_args_in_usage(true)
        .arg(
            Arg::new("version")
                .short('V')
                .short_alias('v')
                .help("Print version numbers and nginx configurations.")
                .action(SetTrue)
        )
        .group(ArgGroup::new("runners"))
        .arg(
            Arg::new("lua-package-path")
                .short('I')
                .value_name("DIR")
                .num_args(1)
                .action(Append)
                .help("Add dir to the search paths for Lua libraries.")
                .value_parser(NonEmptyStringValueParser::new())
                .value_hint(ValueHint::DirPath)
        )
        .arg(
            Arg::new("inline-lua")
                .short('e')
                .num_args(1)
                .value_name("PROG")
                .action(Append)
                .help(r#"Run the inlined Lua code in "prog"."#)
                .value_parser(NonEmptyStringValueParser::new())
        )
        .arg(
            Arg::new("lua-libraries")
                .short('l')
                .value_name("LIB")
                .num_args(1)
                .action(Append)
                .help(r#"require lua library "lib""#)
                .value_parser(NonEmptyStringValueParser::new())
        )
        .arg(
            Arg::new("jit")
                .short('j')
                .num_args(1)
                .value_name("OPT")
                .value_parser(EnumValueParser::<JitCmd>::new())
                .help("LuaJIT option:

-j dump    Use LuaJIT's jit.dump module to output detailed info of
           the traces generated by the JIT compiler.

-j off     Turn off the LuaJIT JIT compiler.

-j v       Use LuaJIT's jit.v module to output brief info of the
           traces generated by the JIT compiler.")

        )
        .arg(
            Arg::new("worker-connections")
                .short('c')
                .help("Set maximal connection count")
                .num_args(1)
                .value_name("NUM")
                .value_parser(value_parser!(u32).range(1..))
                .default_value("64")
        )
        .arg(
            Arg::new("nameservers")
                .long("ns")
                .num_args(1)
                .value_name("IP")
                .action(Append)
                .help("Specify a custom name server (multiple instances are supported).")
                .value_parser(value_parser!(IpAddr))
        )
        .arg(
            Arg::new("shdict")
                .long("shdict")
                .num_args(1)
                .value_name("NAME SIZE")
                .action(Append)
                .help("Create the specified lua shared dicts in the http configuration block (multiple instances are supported).")
                .value_parser(NonEmptyStringValueParser::new())
        )
        .arg(
            Arg::new("nginx-path")
                .long("nginx")
                .value_name("PATH")
                .num_args(1)
                .help("Specify the nginx path (this option might be removed in the future).")
                .value_parser(NonEmptyStringValueParser::new())
                .value_hint(ValueHint::ExecutablePath)
        )
        .arg(
            Arg::new("http-conf")
                .long("http-conf")
                .num_args(1)
                .value_name("CONF")
                .action(Append)
                .help("Specifies nginx.conf snippet inserted into the http {} configuration block (multiple instances are supported).")
                .value_parser(NonEmptyStringValueParser::new())
        )
        .arg(
            Arg::new("stream-conf")
                .long("stream-conf")
                .num_args(1)
                .value_name("CONF")
                .action(Append)
                .help("Disable the stream {} configuration in auto-generated nginx.conf.")
                .value_parser(NonEmptyStringValueParser::new())
        )
        .arg(
            Arg::new("main-conf")
                .long("main-conf")
                .num_args(1)
                .value_name("CONF")
                .action(Append)
                .help("Specifies nginx.conf snippet inserted into the nginx main {} configuration block (multiple instances are supported).")
                .value_parser(NonEmptyStringValueParser::new())
        )
        .arg(
            Arg::new("http-include")
                .long("http-include")
                .num_args(1)
                .value_name("PATH")
                .action(Append)
                .help("Include the specified file in the nginx http configuration block (multiple instances are supported).")
                .value_parser(NonEmptyStringValueParser::new())
                .value_hint(ValueHint::FilePath)
        )
        .arg(
            Arg::new("main-include")
                .long("main-include")
                .num_args(1)
                .value_name("PATH")
                .action(Append)
                .help("Include the specified file in the nginx main configuration block (multiple instances are supported).")
                .value_parser(NonEmptyStringValueParser::new())
                .value_hint(ValueHint::FilePath)
        )
        .arg(
            Arg::new("valgrind")
                .long("valgrind")
                .action(SetTrue)
                .help("Use valgrind to run nginx.")
                .group("runners")
        )
        .arg(
            Arg::new("valgrind-opts")
                .long("valgrind-opts")
                .value_name("OPTS")
                .num_args(1)
                .allow_hyphen_values(true)
                .help("Pass extra options to valgrind.")
        )
        .arg(
            Arg::new("errlog-level")
                .long("errlog-level")
                .value_name("LEVEL")
                .num_args(1)
                .value_parser(EnumValueParser::<LogLevel>::new())
                .help("Set nginx error_log level.")
        )
        .arg(
            Arg::new("resolve-ipv6")
                .long("resolve-ipv6")
                .action(SetTrue)
                .help("Make the nginx resolver lookup both IPv4 and IPv6 addresses.")
        )
        .arg(
            Arg::new("user-runner")
                .long("user-runner")
                .num_args(1)
                .help("Use CMD as user runner for the underlying nginx process.")
        )
        .arg(
            Arg::new("stap")
                .long("stap")
                .action(SetTrue)
                .help("Use sysetmtap to run the underlying nginx C process.")
                .group("runners")
        )
        .arg(
            Arg::new("stap-opts")
                .long("stap-opts")
                .num_args(1)
                .allow_hyphen_values(true)
                .help("Pass extra systemtap command line options.")
        )
        .arg(
            Arg::new("gdb")
                .long("gdb")
                .action(SetTrue)
                .help("Use GDB to run the underlying nginx C process.")
                .group("runners")
        )
        .arg(
            Arg::new("gdb-opts")
                .long("gdb-opts")
                .num_args(1)
                .allow_hyphen_values(true) // this doesn't work
                .help("Pass extra command-line options to GDB.")
        )
        .arg(
            Arg::new("no-stream")
                .long("no-stream")
                .action(SetTrue)
                .help("Specifies nginx.conf snippet inserted into the nginx stream {} configuration block (multiple instances are supported).")
        )
        .arg(
            Arg::new("rr")
                .long("rr")
                .action(SetTrue)
                .help("Use Mozilla rr to record the execution of the underlying nginx C process.")
                .group("runners")
        )
        .arg(
            Arg::new("lua-file")
                .num_args(1)
                .value_name("lua-file")
                .value_parser(NonEmptyStringValueParser::new())
                .value_hint(ValueHint::FilePath)
        )
        .arg(
            Arg::new("lua-args")
                .num_args(1)
                .value_name("args")
                .allow_hyphen_values(true) // this doesn't work
                .action(Append)
        )
}

#[test]
fn verify_app() {
    cmd().debug_assert();
}

pub(crate) fn letsgo() -> Result<App, clap::Error> {
    App::try_from(env::args())
}

#[derive(Default, Debug)]
pub(crate) struct App {
    pub(crate) inline_lua: Vec<String>,
    pub(crate) lua_file: Option<String>,
    pub(crate) lua_args: Vec<String>,

    pub(crate) nginx: PathBuf,
    pub(crate) worker_connections: u32,
    pub(crate) errlog_level: LogLevel,
    pub(crate) lua_package_path: Vec<String>,
    pub(crate) nameservers: Vec<String>,
    pub(crate) resolve_ipv6: bool,

    pub(crate) http_conf: Vec<String>,
    pub(crate) main_conf: Vec<String>,
    pub(crate) stream_conf: Vec<String>,
    pub(crate) no_stream: bool,

    pub(crate) runner: Runner,

    pub(crate) version: bool,

    pub(crate) prefix: Option<String>,
}

impl From<App> for process::Command {
    fn from(app: App) -> Self {
        let root = app.prefix.unwrap();

        // resty CLI always adds a trailing slash
        let prefix = format!("{}/", root.trim_end_matches('/'));

        let nginx = app.nginx.to_str().unwrap().to_owned();
        let mut nginx_args = vec![
            String::from("-p"),
            prefix,
            String::from("-c"),
            String::from("conf/nginx.conf"),
        ];

        let bin: String;
        let mut args: Vec<String> = vec![];

        match app.runner {
            Runner::Default => {
                bin = nginx;
                args.append(&mut nginx_args);
            }
            Runner::RR => {
                bin = String::from("rr");
                args.push(String::from("record"));
                args.push(nginx);
                args.append(&mut nginx_args);
            }
            Runner::Stap(opts) => {
                bin = String::from("stap");
                args = vec![];
                if let Some(opts) = opts {
                    args.append(&mut split_shell_args(&opts));
                }
                args.push("-c".to_owned());
                nginx_args.insert(0, nginx);
                args.push(join_shell_args(
                    nginx_args.iter_mut().map(|s| s.as_str()).collect(),
                ));
            }
            Runner::Valgrind(opts) => {
                bin = "valgrind".to_owned();
                args = vec![];
                if let Some(opts) = opts {
                    args.append(&mut split_shell_args(&opts));
                }
                args.push(nginx);
                args.append(&mut nginx_args);
            }
            Runner::Gdb(opts) => {
                bin = String::from("gdb");
                if let Some(opts) = opts {
                    args.append(&mut split_shell_args(&opts));
                }
                args.push("--args".to_owned());
                args.push(nginx);
                args.append(&mut nginx_args);
            }
            Runner::User(runner) => {
                let mut user_args = split_shell_args(&runner);
                bin = user_args.remove(0);
                args.append(&mut user_args);
                args.push(nginx);
                args.append(&mut nginx_args);
            }
        };

        let mut c = process::Command::new(bin);

        c.args(args);
        c
    }
}

#[derive(Default, Debug)]
pub(crate) enum Runner {
    #[default]
    Default,
    RR,
    Stap(Option<String>),
    Valgrind(Option<String>),
    Gdb(Option<String>),
    User(String),
}

impl From<&mut ArgMatches> for Runner {
    fn from(m: &mut ArgMatches) -> Self {
        if consume_flag("rr", m) {
            return Self::RR;
        } else if consume_flag("stap", m) {
            return Self::Stap(consume_arg_string("stap-opts", m));
        } else if consume_flag("valgrind", m) {
            return Self::Valgrind(consume_arg_string("valgrind-opts", m));
        } else if consume_flag("gdb", m) {
            return Self::Gdb(consume_arg_string("gdb-opts", m));
        } else if let Some(user) = consume_arg_string("user-runner", m) {
            return Self::User(user);
        }

        Self::Default
    }
}

fn zip_indices<'a>(m: &'a ArgMatches, id: &str) -> impl IntoIterator<Item = ValueWithIndex> + 'a {
    let values = m.get_many::<String>(id).unwrap_or_default();

    let indices = m.indices_of(id).unwrap_or_default();

    indices.zip(values).map(ValueWithIndex::from)
}

impl TryFrom<env::Args> for App {
    type Error = clap::Error;

    fn try_from(args: env::Args) -> Result<Self, Self::Error> {
        let mut app = Self::default();

        let mut args = args.collect::<VecDeque<String>>();

        let mut clap_args = vec![];
        clap_args.extend(args.pop_front());

        let mut c = cmd();
        let mut opts: HashMap<String, bool> = HashMap::new();
        for a in c.get_arguments() {
            if a.is_positional() {
                continue;
            }

            let takes_value = match (a.get_id().as_str(), a.get_action()) {
                // for some ungodly reason, -h|--help has a SetValue action
                ("help", _) => false,
                (_, ArgAction::SetTrue) => false,
                (_, ArgAction::SetFalse) => false,
                _ => true,
            };

            if let Some(c) = a.get_short() {
                let _ = opts.insert(c.to_string(), takes_value);
            }

            if let Some(l) = a.get_long() {
                let _ = opts.insert(l.to_string(), takes_value);
            }

            if let Some(al) = a.get_all_aliases() {
                al.iter().for_each(|s| {
                    let _ = opts.insert(s.to_string(), takes_value);
                });
            }
        }

        let takes_value = |arg: &str| -> bool {
            let arg = arg.trim_start_matches('-');
            *opts.get(arg).unwrap_or(&false)
        };

        // pre-parse the CLI args
        //
        // resty-cli allows additional args to be passed to lua. This can take
        // several forms:
        //
        // resty filename.lua <args>
        // resty -e <expr> <args>
        // resty -e <expr> -- <args>
        //
        // Things get tricky when <args> contains elements that look like cli params
        // (e.g. -y|--foo), because clap attempts to parse them and complains that
        // an unknown arg was provided. This occurs even when `allow_hyphen_values`
        // is set on the final, variadic positional arg:
        //
        // https://github.com/clap-rs/clap/issues/1538
        //
        // To work around this problem, we have to pre-process cli args before
        // passing them to the clap parser.
        while let Some(elem) = args.pop_front() {
            // end of input, everything else is a lua arg
            if elem == "--" {
                break;

            // flag or option
            } else if elem.starts_with('-') {
                if takes_value(&elem) {
                    clap_args.push(elem);
                    clap_args.extend(args.pop_front());
                } else {
                    clap_args.push(elem);
                }

            // lua file + optional args here
            } else {
                // let clap handle the lua file
                clap_args.push(elem);
                break;
            }
        }

        app.lua_args.extend(args);

        let mut m = c.try_get_matches_from_mut(clap_args)?;

        app.nginx = find_nginx_bin(consume_arg_string("nginx-path", &mut m));
        app.version = consume_flag("version", &mut m);
        if app.version {
            return Ok(app);
        }

        extend_from_args(&mut app.lua_args, "lua-args", &mut m);

        app.lua_file = consume_arg_string("lua-file", &mut m);

        if let (None, false) = (&app.lua_file, arg_is_present("inline-lua", &m)) {
            if !app.version {
                return Err(c.error(
                    ErrorKind::MissingRequiredArgument,
                    "at least one of -e <expr> or lua file is required\n",
                ));
            }
        }

        if let Some(file) = &app.lua_file {
            if !PathBuf::from(file).exists() {
                let msg = format!("Lua input file {file} not found.\n");
                return Err(c.error(ErrorKind::ValueValidation, msg));
            }
        }

        // the hard part
        //
        // build a Vec<String> of inlined lua expressions
        //
        // expressions can be explicitly specified by the user:
        //
        // resty -e 'print("hello, world")'
        //
        // or they might be generated from other cli args:
        //
        // resty -j off => 'require("jit").off()'
        // resty -l foo => 'require("foo")'
        //
        // the original resty cli generates these expressions in the order that
        // they are found in the cli args, so we need to find their indices in
        // the original cli args and interleave them into a final list of
        // expressions
        //
        // Example:
        //
        // resty -e 'print("first")' \
        //       -l my-library \
        //       -e 'print("third")' \
        //       -j off
        //
        // This should get parsed into:
        //
        // [
        //   "require('jit').off()",
        //   "print('first')",
        //   "require('my-library')",
        //   "print('third'),
        // ]
        //
        // Note that jit options (-j) are always pushed to the front.

        let mut values_with_indices = vec![];

        values_with_indices.extend(zip_indices(&m, "inline-lua"));

        values_with_indices.extend(zip_indices(&m, "lua-libraries").into_iter().map(|vwi| {
            ValueWithIndex {
                value: format!("require({})", quote_lua_string(&vwi.value)),
                index: vwi.index,
            }
        }));

        // jit command (-j dump|off|v) always goes first
        if let Some(jit) = m.get_one::<JitCmd>("jit") {
            app.inline_lua.push(String::from(jit));
        }

        // finally, sort by index and convert to string
        values_with_indices.sort();
        app.inline_lua
            .extend(values_with_indices.into_iter().map(String::from));

        // phew

        nameservers(&mut app, &mut m);
        set_from_args(&mut app.resolve_ipv6, "resolve-ipv6", &mut m);

        set_from_args(&mut app.worker_connections, "worker-connections", &mut m);
        set_from_args(&mut app.no_stream, "no-stream", &mut m);
        set_from_args(&mut app.errlog_level, "errlog-level", &mut m);
        extend_from_args(&mut app.lua_package_path, "lua-package-path", &mut m);

        main_conf(&mut app, &mut m)?;
        http_conf(&mut app, &mut m)?;
        stream_conf(&mut app, &mut m);

        app.runner = Runner::from(&mut m);

        Ok(app)
    }
}
