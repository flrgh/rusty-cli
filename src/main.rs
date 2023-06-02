#[macro_use]
extern crate lazy_static;

mod cli;
mod lua;
mod nginx;
mod run;
mod types;
mod util;

use crate::cli::*;
use std::process::exit;

fn main() {
    match new_parse() {
        Err(e) => {
            eprintln!("{}", e);
            exit(e.exit_code());
        }

        Ok(action) => {
            exit(action.run());
        }
    }
}
