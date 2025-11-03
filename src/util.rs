use crate::types::IpAddr;
use nix::unistd::mkdtemp;
use std::fs;
use std::io::{self, BufRead, BufReader, Read};
use std::path::PathBuf;

fn impl_tempdir(tpl: &str) -> io::Result<PathBuf> {
    Ok(mkdtemp(tpl)?)
}

pub(crate) fn tempdir() -> io::Result<PathBuf> {
    const MKDTEMP_TEMPLATE: &str = "/tmp/resty_XXXXXX";
    impl_tempdir(MKDTEMP_TEMPLATE)
}

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
        // resty-cli only detects IPv4 addresses from resolv.conf
        // https://github.com/openresty/resty-cli/blob/3022948ef3d670b915bcf7027bcdd917591b96e4/bin/resty#L577
        .filter(|addr| addr.is_ipv4())
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
    shlex::split(s.as_ref()).expect("Invalid shell args")
}

pub(crate) fn join_shell_args<T: AsRef<str>>(args: &Vec<T>) -> String {
    let mut out = Vec::with_capacity(args.len());

    // The shlex crate takes a slightly different approach of wrapping the
    // entire string in double quotes and then only escaping a few chars
    // within the string. It's a little bit cleaner, but in the interest of
    // compatibility we'll duplicate the resty-cli algorithm exactly*:
    //
    //   s/([\\\s'"><`\[\]\&\$#*?!()|;])/\\$1/g;
    //
    // *additionally escaping '{' and '}'
    #[rustfmt::skip]
    fn escape(c: u8, buf: &mut Vec<u8>) {
        match c as char {
            '\\'
            | ' ' | '\t' | '\r' | '\n'
            | '\'' | '"' | '`'
            | '<' | '>'
            | '[' | ']'
            | '(' | ')'
            | '{' | '}'
            | '|'
            | ';'
            | '&'
            | '$'
            | '#'
            | '*'
            | '?'
            | '!'
            => {
                buf.push(b'\\');
            }
            _ => {}
        }

        buf.push(c);
    }

    let mut arg_buf = Vec::new();

    for arg in args {
        let arg = arg.as_ref();
        arg_buf.reserve(arg.len() - arg_buf.len());

        for c in arg.bytes() {
            escape(c, &mut arg_buf);
        }

        let bytes = std::mem::take(&mut arg_buf);
        out.push(String::from_utf8(bytes).unwrap());
    }

    out.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;

    #[test]
    fn test_split_shell_args() {
        let tests = vec![
            (r#"-a -b"#, vec!["-a", "-b"]),
            (r#"\(\) \[\] \{\} \<\>"#, vec!["()", "[]", "{}", "<>"]),
            (r#"\\ \" \' \`"#, vec!["\\", "\"", "'", "`"]),
            (
                r#"\& \* \# \$ \? \! \; \|"#,
                vec!["&", "*", "#", "$", "?", "!", ";", "|"],
            ),
            (r#"'single quote'"#, vec!["single quote"]),
            (r#"escaped\ space"#, vec!["escaped space"]),
        ];

        for (input, expect) in tests {
            assert_eq!(expect, split_shell_args(input));
        }
    }

    #[test]
    fn test_join_shell_args() {
        let tests = vec![
            (r#"-a -b"#, vec!["-a", "-b"]),
            (r#"\(\) \[\] \{\} \<\>"#, vec!["()", "[]", "{}", "<>"]),
            (r#"\\ \" \' \`"#, vec!["\\", "\"", "'", "`"]),
            (
                r#"\& \* \# \$ \? \! \; \|"#,
                vec!["&", "*", "#", "$", "?", "!", ";", "|"],
            ),
            (r#"single\ quote"#, vec!["single quote"]),
            (r#"escaped\ space"#, vec!["escaped space"]),
        ];

        for (expect, input) in tests {
            assert_eq!(expect, join_shell_args(&input));
        }
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
        macro_rules! addrs {
            ( $( $x:expr ),* ) => {
                {
                    let v = vec![$($x.parse::<IpAddr>().unwrap(),)*];
                    v
                }
            };
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
            addrs!["127.0.0.53"],
            impl_try_parse_resolv_conf(input.as_bytes())
        );

        let input = "nameserver 127.0.0.53";

        assert_eq!(
            addrs!["127.0.0.53"],
            impl_try_parse_resolv_conf(input.as_bytes())
        );

        let input = r##"
_nameserver 127.0.0.1
nameserver 127.0.0.2
nameserver 127.0.0.3 oops extra stuff
"##;

        assert_eq!(
            addrs!["127.0.0.2"],
            impl_try_parse_resolv_conf(input.as_bytes())
        );

        let input = r##"
nameserver 127.0.0.1
nameserver 127.0.0.2
nameserver 127.0.0.3
"##;

        assert_eq!(
            addrs!["127.0.0.1", "127.0.0.2", "127.0.0.3"],
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
            addrs![
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
                "127.0.0.11"
            ],
            impl_try_parse_resolv_conf(input.as_bytes())
        );

        let input = r##"
nameserver 127.0.0.1
nameserver ::1
nameserver ::1/128
nameserver 0:0:0:0:0:0:0:1
nameserver 127.0.0.3
"##;
        assert_eq!(
            addrs!["127.0.0.1", "127.0.0.3"],
            impl_try_parse_resolv_conf(input.as_bytes())
        );
    }

    #[test]
    fn temp_dir_invalid_template() {
        let res = impl_tempdir("/tmp/weeee");
        assert!(res.is_err());

        let e = res.unwrap_err();
        assert!(matches!(e.kind(), ErrorKind::InvalidInput));
    }

    #[test]
    fn temp_dir_invalid_template_string() {
        let res = impl_tempdir("/tmp/null_\0_foo_XXXXXX");
        assert!(res.is_err());

        let e = res.unwrap_err();
        assert!(matches!(e.kind(), ErrorKind::InvalidInput));
    }

    #[test]
    fn temp_dir_io_error() {
        let res = impl_tempdir("/a/b/i-dont-exist/c/d/e/f/g/foo_XXXXXX");
        assert!(res.is_err());

        let e = res.unwrap_err();
        assert!(matches!(e.kind(), ErrorKind::NotFound));
    }

    #[test]
    fn temp_dir_happy_path() {
        let result = impl_tempdir("/tmp/cargo_test_XXXXXX");
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.is_dir());

        std::fs::remove_dir_all(path).expect("cleanup of temp dir");
    }

    #[test]
    fn resty_version_parser() {
        use crate::compat_version::Version;

        macro_rules! parse {
            ($input:literal => $exp:expr) => {{
                let input = $input;
                let (maj, min) = $exp;
                assert_eq!(
                    Some(Version::new(maj, min)),
                    Version::from_str(input),
                    "Version::from_str(\"{input}\") => {maj}.{min}"
                );
            }};
        }

        macro_rules! no_parse {
            ($input:literal) => {{
                let input = $input;
                assert_eq!(
                    None,
                    Version::from_str(input),
                    "Version::from_str(\"{input}\") should fail"
                );
            }};
        }

        parse!(".1" => (0, 1));
        parse!(".1" => (0, 1));
        parse!(".12" => (0, 12));

        parse!("1" => (1, 0));
        parse!("1.2" => (1, 2));
        parse!("1.2.3" => (1, 2));
        parse!("1.2.3-extra" => (1, 2));
        parse!("v1" => (1, 0));
        parse!("v1.2" => (1, 2 ));
        parse!("v1.2.3" => (1, 2));
        parse!("v1.2.3-extra" => (1, 2));

        parse!("0.1" => (0, 1));
        parse!("0.1.2" => (0, 1));
        parse!("0.1.2" => (0, 1));
        parse!("0.1.2-extra" => (0, 1));
        parse!("v0.1" => (0, 1));
        parse!("v0.1.2" => (0, 1));
        parse!("v0.1.2" => (0, 1));
        parse!("v0.1.2-extra" => (0, 1));

        parse!("0.12" => (0, 12));
        parse!("0.12.34" => (0, 12));
        parse!("0.12.34" => (0, 12));
        parse!("0.12.34-extra" => (0, 12));
        parse!("v0.12" => (0, 12));
        parse!("v0.12.34" => (0, 12));
        parse!("v0.12.34" => (0, 12));
        parse!("v0.12.34-extra" => (0, 12));

        parse!("0.123" => (0, 123));
        parse!("0.123.456" => (0, 123));
        parse!("0.123.456" => (0, 123));
        parse!("0.123.456-extra" => (0, 123));
        parse!("v0.123" => (0, 123));
        parse!("v0.123.456" => (0, 123));
        parse!("v0.123.456" => (0, 123));
        parse!("v0.123.456-extra" => (0, 123));

        no_parse!("");
        no_parse!("v");
        no_parse!("-1");
        no_parse!("vv1");
        no_parse!("0..1");
        no_parse!("..1");
        no_parse!("vv1");

        no_parse!("999999999999999999999");
        no_parse!("9999999999999999999999999");
        no_parse!("1000000000000000000000000000000");

        parse!("10000" => (10_000, 0));
        parse!("10000.10000" => (10_000, 10_000));
        no_parse!("100000");

        parse!("65535" => (65_535, 0));
        parse!("65535.65535" => (65_535, 65_535));
        parse!("0.65535" => (0, 65_535));
        no_parse!("65536");

        no_parse!("00000000000000");
    }
}
