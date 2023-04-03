use clap;
use errno::errno;
use libc::{c_char, c_int, int8_t, mkdtemp, size_t, PT_NULL};
use mktemp::Temp;
use std::ffi;
use std::ffi::CStr;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::fs;
use std::io;
use std::net;
use std::path::PathBuf;
use std::string::ToString;
use strum_macros;

const MKDTEMP_TEMPLATE: &str = "/tmp/resty_XXXXXX";

pub fn tempdir(tpl: Option<&str>) -> io::Result<String> {
    let tpl = ffi::CString::new(tpl.unwrap_or(MKDTEMP_TEMPLATE)).unwrap();
    unsafe {
        let res = mkdtemp(tpl.as_ptr() as *mut c_char);

        if res == std::ptr::null_mut() {
            let e = errno();
            return Err(io::Error::from_raw_os_error(e.0));
        }

        Ok(ffi::CStr::from_ptr(res).to_str().unwrap().to_string())
        //Ok("yes".to_string())

        //Ok(std::ffi::CString::from_raw(res).to_str().unwrap().to_owned())
    }
    //    todo!();
}

#[test]
fn pls_dont_die() {
    //assert_eq!("abcd".to_string(), tempdir(None).unwrap());
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

impl std::str::FromStr for IpAddr {
    type Err = net::AddrParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        trim_brackets(s)
            .parse::<net::IpAddr>()
            .map(|_| IpAddr(s.to_owned()))
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

#[derive(
    clap::ValueEnum, Clone, Debug, Default, strum_macros::Display, strum_macros::EnumString,
)]
#[clap(rename_all = "lower")]
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

#[derive(clap::ValueEnum, Clone, Debug, strum_macros::EnumString)]
#[clap(rename_all = "lower")]
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

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ValueWithIndex {
    pub(crate) value: String,
    pub(crate) index: usize,
}

impl PartialOrd for ValueWithIndex {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.index.partial_cmp(&other.index)
    }
}

impl Ord for ValueWithIndex {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.index.cmp(&other.index)
    }
}

impl From<(usize, &String)> for ValueWithIndex {
    fn from(idx_val: (usize, &String)) -> Self {
        ValueWithIndex {
            index: idx_val.0,
            value: idx_val.1.to_owned(),
        }
    }
}

impl From<ValueWithIndex> for String {
    fn from(val: ValueWithIndex) -> Self {
        val.value
    }
}
