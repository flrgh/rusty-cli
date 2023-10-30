use std::io::prelude::*;
use serde_json;
use std::process::{Command, Stdio};
use regex::Regex;


pub static BIN: &str = "./target/debug/rusty-cli";
pub static RESTY: &str = "./resty-cli/bin/resty";

#[derive(Debug, PartialEq, Eq)]
pub struct CmdResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub fn patch_result(result: &str) -> String {
    let re = Regex::new("/tmp/resty_[^/]+").unwrap();
    let result = re.replace_all(result, "<TMP>");
    result
        .replace(BIN, "<BIN>")
        .replace(RESTY, "<BIN>")
}

pub fn run(bin: &str, args: &Vec<&str>) -> CmdResult {
    let mut cmd = Command::new(bin);
    cmd.env("RUSTY_STRIP_LUA_INDENT", "1");
    cmd.args(args.clone());
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut proc = cmd.spawn().expect("failed to spawn command");
    let status = proc.wait().expect("failed to wait for command to finish");

    let mut buf = String::new();
    let mut stdout = proc.stdout.expect("failed to get stdout handle");

    stdout.read_to_string(&mut buf).expect("failed to read stdout");
    let stdout = patch_result(&buf);

    let mut stderr = proc.stderr.expect("failed to get stderr handle");
    buf.clear();
    stderr.read_to_string(&mut buf).expect("failed to read stderr");
    let stderr = patch_result(&buf);

    CmdResult {
        exit_code: status.code().unwrap_or(0),
        stdout,
        stderr,
    }
}


pub fn run_both(args: &Vec<&str>) -> (CmdResult, CmdResult) {
    (run(RESTY, args), run(BIN, args))
}

pub fn assert_same(args: &Vec<&str>) {
    let (resty, rusty) = run_both(args);
    similar_asserts::assert_eq!(resty, rusty);
}

pub fn json_decode(s: &str) -> serde_json::Value {
    serde_json::from_str(s).expect("invalid JSON")
}
