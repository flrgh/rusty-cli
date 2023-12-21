mod cli;
mod lua;
mod nginx;
mod run;
mod types;
mod util;

use std::process::exit;

fn main() {
    match cli::Action::try_from(std::env::args()) {
        Err(e) => {
            eprintln!("{}", e);
            exit(e.exit_code());
        }

        Ok(action) => {
            exit(action.run());
        }
    }
}
