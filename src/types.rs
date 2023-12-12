use libc::{c_char, mkdtemp};
use std::ffi;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::fs;
use std::io;
use std::net;
use std::path::PathBuf;
use std::string::ToString;
use thiserror::Error as ThisError;

const MKDTEMP_TEMPLATE: &str = "/tmp/resty_XXXXXX";

pub fn tempdir(tpl: Option<&str>) -> io::Result<String> {
    let tpl = ffi::CString::new(tpl.unwrap_or(MKDTEMP_TEMPLATE)).unwrap();
    unsafe {
        let res = mkdtemp(tpl.as_ptr() as *mut c_char);

        if res.is_null() {
            return Err(std::io::Error::last_os_error());
        }

        Ok(ffi::CStr::from_ptr(res).to_str().unwrap().to_string())
    }
}

#[test]
fn pls_dont_die() {
    assert!(tempdir(Some("/tmp/weeee")).is_err());
    assert!(tempdir(None).is_ok());
}

fn trim_brackets(s: &str) -> &str {
    s.strip_prefix('[')
        .and_then(|st| st.strip_suffix(']'))
        .unwrap_or(s)
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct IpAddr(String);

impl Display for IpAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.0)
    }
}

impl From<IpAddr> for String {
    fn from(val: IpAddr) -> Self {
        val.0
    }
}

impl std::str::FromStr for IpAddr {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        trim_brackets(s)
            .parse::<net::IpAddr>()
            .map(|_| IpAddr(s.to_owned()))
            .map_err(|_| "expecting an IP address".to_string())
    }
}

#[test]
fn test_ip_addr_from_str() {
    assert_eq!(Ok(IpAddr("[::1]".to_string())), "[::1]".parse::<IpAddr>());

    assert_eq!(
        Ok(IpAddr("[2003:dead:beef:4dad:23:46:bb:101]".to_string())),
        "[2003:dead:beef:4dad:23:46:bb:101]".parse::<IpAddr>()
    );

    assert_eq!(
        Ok(IpAddr("127.0.0.1".to_string())),
        "127.0.0.1".parse::<IpAddr>()
    );
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct Shdict(String);

impl From<Shdict> for String {
    fn from(val: Shdict) -> Self {
        val.0
    }
}

impl Display for Shdict {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.0)
    }
}

impl Shdict {
    pub(crate) fn to_nginx(&self) -> String {
        format!("lua_shared_dict {};", self.0)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct InvalidShdict(String);

impl From<&str> for InvalidShdict {
    fn from(input: &str) -> Self {
        InvalidShdict(input.to_string())
    }
}

impl Display for InvalidShdict {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "expecting NAME SIZE")
    }
}

impl std::error::Error for InvalidShdict {}

impl std::str::FromStr for Shdict {
    type Err = InvalidShdict;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with(' ') || s.ends_with(' ') {
            return Err(InvalidShdict::from(s));
        }

        let mut parts = s.split_whitespace();

        let name = parts.next().ok_or_else(|| InvalidShdict::from(s))?;
        let size = parts.next().ok_or_else(|| InvalidShdict::from(s))?;

        //dbg!(s, name, size, &parts);

        if parts.next().is_some() {
            return Err(InvalidShdict::from(s));
        }

        // yeah this is all kinda ugly, but I don't want the regex crate

        for c in name.chars() {
            if !c.is_ascii_alphanumeric() && c != '_' {
                return Err(InvalidShdict::from(s));
            }
        }

        size.strip_suffix(['k', 'K', 'm', 'M'])
            .unwrap_or(size)
            .parse::<u32>()
            .map_err(|_| InvalidShdict::from(s))?;

        Ok(Shdict(format!("{} {}", name, size)))
    }
}

#[test]
fn shdict_from_str() {
    fn shdict(name: &str, size: &str) -> Shdict {
        Shdict(format!("{} {}", name, size))
    }

    fn must_parse(input: &str, (name, size): (&str, &str)) {
        assert_eq!(Ok(shdict(name, size)), input.parse::<Shdict>())
    }

    fn must_not_parse(s: &str) {
        assert_eq!(Err(InvalidShdict::from(s)), s.parse::<Shdict>())
    }

    must_parse("foo_1 10k", ("foo_1", "10k"));
    must_parse("foo_1 10K", ("foo_1", "10K"));
    must_parse("foo_1 10m", ("foo_1", "10m"));
    must_parse("foo_1 10M", ("foo_1", "10M"));
    must_parse("cats_dogs   20000", ("cats_dogs", "20000"));

    must_not_parse("");
    must_not_parse("foo 10 extra");
    must_not_parse("foo 10 extra extra");
    must_not_parse("- 10");
    must_not_parse("   foo 10");
    must_not_parse("foo");
    must_not_parse("foo f");
    must_not_parse("foo 10.0");
    must_not_parse("foo 10b");
    must_not_parse("foo 10g");
    must_not_parse("foo 10km");
    must_not_parse("fo--o 10k");
    must_not_parse("foo -10k");
    must_not_parse("foo -10");
}

#[derive(Clone, Debug, Default, strum_macros::Display, strum_macros::EnumString)]
#[strum(serialize_all = "lowercase")]
pub(crate) enum LogLevel {
    Debug,
    Info,
    Notice,
    #[default]
    Warn,
    Error,
    Crit,
    Alert,
    Emerg,
}

#[derive(Clone, Debug, strum_macros::EnumString)]
#[strum(serialize_all = "lowercase")]
pub(crate) enum JitCmd {
    /// Use LuaJIT's jit.v module to output brief info of the
    /// traces generated by the JIT compiler.
    V,

    /// Use LuaJIT's jit.dump module to output detailed info of
    /// the traces generated by the JIT compiler.
    Dump,

    /// Turn off the LuaJIT JIT compiler.
    Off,
}

impl JitCmd {
    pub(crate) fn to_lua(&self) -> String {
        match self {
            JitCmd::V => r#"require "jit.v".on()"#,
            JitCmd::Dump => r#"require "jit.dump".on()"#,
            JitCmd::Off => r#"require "jit".off()"#,
        }
        .to_string()
    }
}

impl From<&JitCmd> for String {
    fn from(val: &JitCmd) -> Self {
        val.to_lua()
    }
}

pub(crate) struct Prefix {
    pub(crate) root: PathBuf,
    pub(crate) conf: PathBuf,
}

impl Debug for Prefix {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.root.to_str().unwrap())
    }
}

impl Display for Prefix {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.root.to_str().unwrap())
    }
}

impl Prefix {
    pub(crate) fn new() -> Result<Self, std::io::Error> {
        let tmp = tempdir(None)?;

        let root = PathBuf::from(tmp);
        let conf = root.join("conf");

        fs::create_dir_all(&root)?;
        fs::create_dir_all(&conf)?;
        fs::create_dir_all(root.join("logs"))?;

        Ok(Prefix { root, conf })
    }
}

impl Drop for Prefix {
    fn drop(&mut self) {
        if let Err(e) = fs::remove_dir_all(&self.root) {
            eprintln!("Failed to remove directory {}: {}", self.root.display(), e);
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct Buf {
    lines: Vec<String>,
    indent: usize,
}

impl Buf {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn newline(&mut self) {
        self.lines.push(String::new());
    }

    pub fn append(&mut self, s: &str) {
        let mut line = String::new();

        if self.indent > 0 {
            line.push_str("    ".repeat(self.indent).as_str());
        }

        line.push_str(s);
        self.lines.push(line);
    }

    pub fn finalize(self) -> Vec<String> {
        self.lines
    }

    pub fn indent(&mut self) {
        self.indent += 1
    }

    pub fn dedent(&mut self) {
        assert!(self.indent > 0);
        self.indent -= 1
    }
}

#[derive(ThisError, Debug)]
pub enum ArgError {
    #[error("ERROR: could not find {0} include file '{1}'")]
    MissingInclude(String, String),

    #[error("ERROR: options {0} and {1} cannot be specified at the same time.")]
    Conflict(String, String),

    #[error("ERROR: Invalid {arg} option value: {value}\n  ({err})")]
    InvalidValue {
        arg: String,
        value: String,
        err: String,
    },

    #[error("unknown argument: `{0}`")]
    UnknownArgument(String),

    #[error("option {0} takes an argument but found none.")]
    MissingValue(String),

    #[error("Neither Lua input file nor -e \"\" option specified.")]
    NoLuaInput,

    #[error("duplicate {0} options")]
    Duplicate(String),

    #[error("Lua input file {0} not found.")]
    LuaFileNotFound(String),
}

impl ArgError {
    pub fn exit_code(&self) -> i32 {
        match self {
            // I/O error
            Self::MissingInclude(_, _) => 2,

            // yup, resty-cli returns 25 (ENOTTY) for mutually-exclusive
            // arguments
            //
            // not on purpose though, it's just a side effect of errno
            // having been set from a previous and unrelated error
            Self::Conflict(_, _) => 25,

            Self::UnknownArgument(_) => 1,

            Self::InvalidValue {
                arg: _,
                value: _,
                err: _,
            } => 255,
            Self::MissingValue(_) => 255,

            Self::NoLuaInput => 2,
            Self::LuaFileNotFound(_) => 2,

            Self::Duplicate(_) => 255,
        }
    }
}
