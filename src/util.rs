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

pub(crate) fn split_shell_args<T: AsRef<str> + ?Sized>(s: &T) -> Vec<String> {
    shlex::split(s.as_ref()).expect("Invalid runner options")
}

pub(crate) fn join_shell_args<T: AsRef<str>>(args: &Vec<T>) -> String {
    let mut out = Vec::with_capacity(args.len());

    // The shlex crate takes a slightly different approach of wrapping the
    // entire string in double quotes and then only escaping a few chars
    // within the string. It's a little bit cleaner, but in the interest of
    // compatibility we'll duplicate the resty-cli algorithm exactly:
    //
    //   s/([\\\s'"><`\[\]\&\$#*?!()|;])/\\$1/g;
    //
    for arg in args {
        let mut new = Vec::new();

        for c in arg.as_ref().bytes() {
            match c as char {
                '\\' | ' ' | '\t' | '\r' | '\n' | '\'' | '"' | '`' | '<' | '>' | '[' | ']'
                | '(' | ')' | '|' | ';' | '&' | '$' | '#' | '*' | '?' | '!' => {
                    new.push(b'\\');
                }
                _ => {}
            }

            new.push(c);
        }

        out.push(String::from_utf8(new).unwrap());
    }

    out.join(" ")
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
            "--nx -batch -ex b\\ main -ex run -ex bt -ex b\\ lj_cf_io_method_write -ex c -ex bt",
            join_shell_args(&vec![
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

    #[test]
    fn test_args_round_trip() {
        let args = vec![
            "--nx",
            "-batch",
            "-ex",
            "b main",
            "--test",
            "!",
            "--test",
            "($",
            "'\\\\\\\"",
            "`echo 123`",
        ];

        let joined = join_shell_args(&args);
        let split = split_shell_args(&joined);
        let rejoined = join_shell_args(&split);
        let resplit = split_shell_args(&rejoined);
        assert_eq!(joined, rejoined);
        assert_eq!(args, resplit);
    }
}
