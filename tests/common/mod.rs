use std::io::prelude::*;
use serde_json;
use std::process::{Command, Stdio};
use regex::Regex;
use std::collections::HashMap;


#[macro_export]
macro_rules! str {
    ( $x:literal ) => {
        String::from($x)
    }
}

pub static RUSTY: &str = "./target/debug/rusty-cli";
pub static RESTY: &str = "./resty-cli/bin/resty";

pub type Env = Option<HashMap<String, String>>;
pub type Args = Vec<String>;

pub fn strings(input: Vec<&str>) -> Args {
    input.iter().map(|s| s.to_string()).collect::<Args>()
}


#[derive(Debug, PartialEq, Eq)]
pub struct CmdResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub stdout_json: serde_json::Value,
}

pub fn patch_result(result: &str) -> String {
    let re = Regex::new("/tmp/resty_[^/]+").unwrap();
    let result = re.replace_all(result, "<TMP>");
    result
        .replace(RUSTY, "<BIN>")
        .replace(RESTY, "<BIN>")
}

pub fn run(bin: &str, args: Args, env: Env) -> CmdResult {
    let mut cmd = Command::new(bin);

    if let Ok(version) = std::env::var("RESTY_CLI_COMPAT_VERSION") {
        cmd.env("RESTY_CLI_COMPAT_VERSION", version);
    };

    if let Some(env) = env {
        cmd.envs(env);
    }

    cmd.args(args.clone());
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());


    let mut proc = cmd.spawn().expect("failed to spawn command");
    let status = proc.wait().expect("failed to wait for command to finish");

    let mut buf = String::new();
    let mut stdout = proc.stdout.expect("failed to get stdout handle");

    stdout.read_to_string(&mut buf).expect("failed to read stdout");
    let mut stdout = patch_result(&buf);

    let mut stderr = proc.stderr.expect("failed to get stderr handle");
    buf.clear();
    stderr.read_to_string(&mut buf).expect("failed to read stderr");
    let stderr = patch_result(&buf);

    let stdout_json = match serde_json::from_str(&stdout) {
        Ok(value) => {
            stdout = String::from("<JSON>");
            value
        },
        Err(_) => serde_json::Value::Null,
    };

    CmdResult {
        exit_code: status.code().unwrap_or(0),
        stdout,
        stderr,
        stdout_json,
    }
}


pub fn run_both(args: &Args, env: &Env) -> (CmdResult, CmdResult) {
    let mut resty = None;
    let mut rusty = None;

    std::thread::scope(|scope| {
        scope.spawn(|| resty.insert(run(RESTY, args.clone(), env.clone())));
        scope.spawn(|| rusty.insert(run(RUSTY, args.clone(), env.clone())));
    });

    (resty.unwrap(), rusty.unwrap())

}

pub fn assert_same(args: &Args, env: &Env) {
    let (resty, rusty) = run_both(args, env);
    similar_asserts::assert_eq!(resty, rusty, "args: {args:?}, env: {env:?}");
}

pub fn json_decode(s: &str) -> serde_json::Value {
    serde_json::from_str(s).expect("invalid JSON")
}

pub struct TempDir(pub std::path::PathBuf);

use std::ops::Deref;

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(self.0.clone());
    }
}

impl TempDir {
    pub fn new() -> Self {
        Self(std::env::temp_dir())
    }
}

impl Deref for TempDir {
    type Target = std::path::PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
