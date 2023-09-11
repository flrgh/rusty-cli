use crate::types::IpAddr;
use std::fs;
use std::io::{BufRead, BufReader};

pub(crate) fn try_parse_resolv_conf() -> Vec<IpAddr> {
    let mut nameservers = vec![];

    if let Ok(file) = fs::File::open("/etc/resolv.conf") {
        BufReader::new(file)
            .lines()
            .map_while(Result::ok)
            .for_each(|line| {
                let line = line.trim();
                let mut parts = line.split_whitespace();

                let predicate = match parts.next() {
                    Some("nameserver") => parts.next(),
                    _ => return,
                };

                let ns = match predicate {
                    Some(s) => s,
                    // not enough parts
                    _ => return,
                };

                // too many parts
                if parts.next().is_some() {
                    return;
                }

                if let Ok(addr) = ns.parse::<IpAddr>() {
                    nameservers.push(addr);
                }
            });
    }

    nameservers
}

pub fn split_shell_args(s: &str) -> Vec<String> {
    shlex::split(s).expect("Invalid runner options")
}

pub fn join_shell_args(args: Vec<&str>) -> String {
    shlex::join(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_shell_args() {
        assert_eq!(vec![
                "--nx",
                "-batch",
                "-ex",
                "b main",
                "-ex",
                "run",
                "-ex",
                "bt",
                "-ex",
                "b lj_cf_io_method_write",
                "-ex",
                "c",
                "-ex",
                "bt",
            ],
            split_shell_args("--nx -batch -ex 'b main' -ex run -ex bt -ex 'b lj_cf_io_method_write' -ex c -ex bt")
        );

        assert_eq!(vec![
                "--nx",
                "-batch",
                "-ex",
                "b main",
                "-ex",
                "run",
                "-ex",
                "bt",
                "-ex",
                "b lj_cf_io_method_write",
                "-ex",
                "c",
                "-ex",
                "bt",
            ],
            split_shell_args(" --nx -batch -ex 'b main' -ex run -ex bt -ex 'b lj_cf_io_method_write' -ex c -ex bt  ")
        );
    }

    #[test]
    fn test_join_shell_args() {
        assert_eq!(
            "--nx -batch -ex \"b main\" -ex run -ex bt -ex \"b lj_cf_io_method_write\" -ex c -ex bt",
            join_shell_args(vec![
                "--nx",
                "-batch",
                "-ex",
                "b main",
                "-ex",
                "run",
                "-ex",
                "bt",
                "-ex",
                "b lj_cf_io_method_write",
                "-ex",
                "c",
                "-ex",
                "bt",
            ])
        );
    }
}
