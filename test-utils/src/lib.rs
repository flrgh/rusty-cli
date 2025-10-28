use nix::errno::Errno;
use nix::unistd::mkdtemp;
pub use nix::{
    sys::signal::{
        Signal::{
            self, SIGHUP, SIGINT, SIGKILL, SIGPIPE, SIGQUIT, SIGSEGV, SIGTERM, SIGUSR1, SIGUSR2,
            SIGWINCH,
        },
        kill,
    },
    unistd::Pid,
};
pub use std::io::prelude::*;
pub use std::{
    fs::{self, File},
    os::unix::prelude::MetadataExt,
    path::PathBuf,
    str::FromStr,
};
use std::{process::Child, thread::sleep, time::Duration};
pub mod sigscript;
pub use macros::*;

pub trait ToPid {
    fn pid(&self) -> Pid;
}

impl ToPid for Pid {
    fn pid(&self) -> Pid {
        self.to_owned()
    }
}

impl ToPid for Child {
    fn pid(&self) -> Pid {
        Pid::from_raw(self.id() as i32)
    }
}

impl ToPid for &Child {
    fn pid(&self) -> Pid {
        Pid::from_raw(self.id() as i32)
    }
}

impl ToPid for i32 {
    fn pid(&self) -> Pid {
        Pid::from_raw(*self)
    }
}

impl ToPid for u32 {
    fn pid(&self) -> Pid {
        Pid::from_raw(*self as i32)
    }
}

#[derive(Debug)]
pub struct Proc(Pid);

impl Proc {
    pub fn exists(&self) -> bool {
        let res = unsafe { libc::kill(self.0.as_raw(), 0) };
        if res == 0 {
            return true;
        }
        assert_eq!(-1, res, "unexpected return value from kill()");
        match Errno::last() {
            Errno::ESRCH => false,
            e => {
                panic!("kill({}, 0) => unexpected errno ({})", self.0.as_raw(), e);
            }
        }
    }
}

impl Drop for Proc {
    fn drop(&mut self) {
        if self.exists() {
            eprintln!("SIGTERM -> {}", self.0);
            let _ = kill(self.0, SIGTERM);
            sleep_ms(10);

            if self.exists() {
                eprintln!("SIGKILL -> {}", self.0);
                let _ = kill(self.0, SIGKILL);
            }
        }
    }
}

impl<T: ToPid> From<T> for Proc {
    fn from(value: T) -> Self {
        Self(value.pid())
    }
}

#[derive(Debug)]
pub struct TmpDir(PathBuf);

impl TmpDir {
    pub fn new() -> Self {
        let tpl = std::env::temp_dir().join("rusty_cli_test_XXXXXX");
        let dir = mkdtemp(&tpl).expect("create temporary directory");
        Self(dir)
    }

    pub fn path(&self) -> &PathBuf {
        &self.0
    }

    pub fn join<P: Into<PathBuf>>(&self, part: P) -> PathBuf {
        self.0.join(part.into())
    }
}

impl Default for TmpDir {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TmpDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

impl AsRef<PathBuf> for TmpDir {
    fn as_ref(&self) -> &PathBuf {
        &self.0
    }
}

pub fn tmpdir() -> TmpDir {
    TmpDir::new()
}

pub fn sleep_ms(ms: u64) {
    sleep(Duration::from_millis(ms));
}

pub fn file_is_non_empty<P: Into<PathBuf>>(path: P) -> bool {
    let p = path.into();
    fs::metadata(p).is_ok_and(|st| st.size() > 0)
}

pub fn wait_file_contents<P: Into<PathBuf>>(path: P) -> String {
    let p = path.into();
    while !file_is_non_empty(&p) {
        sleep_ms(10);
    }

    let mut f = File::open(p).expect("open file for reading");
    let mut buf = String::new();
    f.read_to_string(&mut buf).expect("read file");
    buf
}

pub fn get_pid<T: std::io::Read>(t: &mut T) -> Pid {
    let mut buf = String::new();
    let _ = t.read_to_string(&mut buf).expect("reading file");
    let num = buf.parse::<i32>().expect("parse pid number");
    Pid::from_raw(num)
}

pub fn wait_pidfile<P: Into<PathBuf>>(path: P) -> Pid {
    let data = wait_file_contents(path);
    let num = data.parse::<i32>().expect("parse pid number");
    Pid::from_raw(num)
}

pub fn cleanup_proc<P: Into<Proc>>(p: P) -> Proc {
    p.into()
}

pub fn lines(bytes: Vec<u8>) -> Vec<String> {
    let data = String::try_from(bytes).expect("invalid utf8 bytes");
    data.split('\n')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned())
        .collect()
}

pub const fn str_to_bool(s: &str) -> Option<bool> {
    if s.eq_ignore_ascii_case("true")
        || s.eq_ignore_ascii_case("yes")
        || s.eq_ignore_ascii_case("1")
        || s.eq_ignore_ascii_case("on")
        || s.eq_ignore_ascii_case("enable")
        || s.eq_ignore_ascii_case("enabled")
    {
        Some(true)
    } else if s.eq_ignore_ascii_case("false")
        || s.eq_ignore_ascii_case("no")
        || s.eq_ignore_ascii_case("0")
        || s.eq_ignore_ascii_case("off")
        || s.eq_ignore_ascii_case("disable")
        || s.eq_ignore_ascii_case("disabled")
    {
        Some(false)
    } else {
        None
    }
}

#[macro_export]
macro_rules! touch {
    ($name:expr) => {{
        let _ = File::create_new($name).expect("create file");
    }};

    ($name:expr, $data:expr) => {{
        use std::io::Write;
        let mut file = File::create_new($name).expect("create file");
        write!(file, "{}", $data).expect("write to file");
        drop(file);
    }};
}
