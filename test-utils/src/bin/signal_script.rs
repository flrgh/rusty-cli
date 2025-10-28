#![allow(static_mut_refs)]

use std::{process, str::FromStr};

use test_utils::sigscript::*;

fn main() {
    let actions = std::env::var("ACTIONS").expect("`ACTIONS` env var must be set");

    let Ok(script) = Script::from_str(&actions) else {
        process::exit(127);
    };

    process::exit(script.exec());
}
