use crate::types::IpAddr;
use std::fs;
use std::io::{BufRead, BufReader, Read};

fn impl_try_parse_resolv_conf<T: Read>(buf: T) -> Vec<IpAddr> {
    BufReader::new(buf)
        .lines()
        .map_while(Result::ok)
        .filter_map(|line| {
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                return None;
            }

            let mut parts = line.split_whitespace();

            match (parts.next(), parts.next(), parts.next()) {
                (Some("nameserver"), Some(addr), None) => addr.parse::<IpAddr>().ok(),
                _ => None,
            }
        })
        .take(11) // resty-cli stops adding nameservers after it has > 10
        .collect()
}

pub(crate) fn try_parse_resolv_conf() -> Vec<IpAddr> {
    let Ok(file) = fs::File::open("/etc/resolv.conf") else {
        return vec![];
    };

    impl_try_parse_resolv_conf(file)
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

    #[test]
    fn test_impl_try_parse_resolv_conf() {
        fn addr<T: AsRef<str>>(s: T) -> IpAddr {
            s.as_ref().parse().unwrap()
        }

        fn addrs<T: AsRef<str>>(s: Vec<T>) -> Vec<IpAddr> {
            s.iter().map(addr).collect()
        }

        let input = r##"# This is /run/systemd/resolve/stub-resolv.conf managed by man:systemd-resolved(8).
# Do not edit.
#
# This file might be symlinked as /etc/resolv.conf. If you're looking at
# /etc/resolv.conf and seeing this text, you have followed the symlink.
#
# This is a dynamic resolv.conf file for connecting local clients to the
# internal DNS stub resolver of systemd-resolved. This file lists all
# configured search domains.
#
# Run "resolvectl status" to see details about the uplink DNS servers
# currently in use.
#
# Third party programs should typically not access this file directly, but only
# through the symlink at /etc/resolv.conf. To manage man:resolv.conf(5) in a
# different way, replace this symlink by a static file or a different symlink.
#
# See man:systemd-resolved.service(8) for details about the supported modes of
# operation for /etc/resolv.conf.

nameserver 127.0.0.53
options edns0 trust-ad
search example.com"##;

        assert_eq!(
            vec![addr("127.0.0.53")],
            impl_try_parse_resolv_conf(input.as_bytes())
        );

        let input = "nameserver 127.0.0.53";

        assert_eq!(
            vec![addr("127.0.0.53")],
            impl_try_parse_resolv_conf(input.as_bytes())
        );

        let input = r##"
_nameserver 127.0.0.1
nameserver 127.0.0.2
nameserver 127.0.0.3 oops extra stuff
"##;

        assert_eq!(
            vec![addr("127.0.0.2")],
            impl_try_parse_resolv_conf(input.as_bytes())
        );

        let input = r##"
nameserver 127.0.0.1
nameserver 127.0.0.2
nameserver 127.0.0.3
"##;

        assert_eq!(
            addrs(vec!["127.0.0.1", "127.0.0.2", "127.0.0.3",]),
            impl_try_parse_resolv_conf(input.as_bytes())
        );

        let input = r##"
nameserver 127.0.0.1
nameserver 127.0.0.2
nameserver 127.0.0.3
nameserver 127.0.0.4
nameserver 127.0.0.5
nameserver 127.0.0.6
nameserver 127.0.0.7
nameserver 127.0.0.8
nameserver 127.0.0.9
nameserver 127.0.0.10
nameserver 127.0.0.11
nameserver 127.0.0.12
"##;

        assert_eq!(
            addrs(vec![
                "127.0.0.1",
                "127.0.0.2",
                "127.0.0.3",
                "127.0.0.4",
                "127.0.0.5",
                "127.0.0.6",
                "127.0.0.7",
                "127.0.0.8",
                "127.0.0.9",
                "127.0.0.10",
                "127.0.0.11",
            ]),
            impl_try_parse_resolv_conf(input.as_bytes())
        );
    }
}
