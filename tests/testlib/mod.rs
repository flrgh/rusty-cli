#![allow(dead_code)]

pub use std::{
    path::PathBuf,
    process::{Command, Stdio},
};
pub use test_utils::*;

pub const RUSTY_PATH: &str = env!("CARGO_BIN_EXE_rusty-cli");
pub const RESTY_PATH: &str = "./resty-cli/bin/resty";

#[derive(Debug, Clone, Copy)]
pub enum Bin {
    Rusty,
    Resty,
}

impl Bin {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rusty => RUSTY_PATH,
            Self::Resty => RESTY_PATH,
        }
    }

    pub fn path(&self) -> PathBuf {
        PathBuf::from(self.as_str())
    }

    pub fn cmd(&self) -> Command {
        let mut cmd = Command::new(self.path());

        // stdout + stderr captured by default
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        cmd.env_clear();
        if let Some(path) = std::env::var_os("PATH") {
            cmd.env("PATH", path);
        }

        cmd
    }

    pub fn is_resty(&self) -> bool {
        !self.is_rusty()
    }

    pub fn is_rusty(&self) -> bool {
        matches!(self, Self::Rusty)
    }

    pub fn exists(&self) -> bool {
        self.path().exists()
    }
}

impl std::fmt::Display for Bin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

pub const RUSTY: Bin = Bin::Rusty;
pub const RESTY: Bin = Bin::Resty;

#[derive(Debug, Clone)]
pub struct TestBin(String);

impl TestBin {
    pub fn new(name: &str) -> Self {
        let path = PathBuf::from_iter(["test-utils", "Cargo.toml"]);
        let mut tb = test_binary::TestBinary::relative_to_parent(name, &path);
        let bin = tb.build().expect("build test binary");

        Self(bin.to_string_lossy().to_string())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }
}

pub fn testbin(name: &str) -> TestBin {
    TestBin::new(name)
}

#[macro_export]
macro_rules! each {
    ($name:ident, $test:expr) => {
        pub mod $name {
            use super::testlib;
            use super::*;

            #[test]
            fn resty() {
                if let Some(true) = testlib::RESTY_ENABLED {
                    assert!(RESTY.exists(), "resty-cli not found");
                    $test(testlib::RESTY);
                }
            }

            #[test]
            fn rusty() {
                $test(testlib::RUSTY);
            }
        }
    };
}
