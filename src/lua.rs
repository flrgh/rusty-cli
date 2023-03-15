use crate::types::*;
use std::cmp::max;
use std::env;
use std::fs;
use std::io::Write as IoWrite;

#[derive(Default)]
struct Buf {
    lines: Vec<String>,
    indent: usize,
}

impl Buf {
    fn new() -> Self {
        Self::default()
    }

    fn newline(&mut self) {
        self.lines.push(String::new());
    }

    fn append(&mut self, s: &str) {
        let mut line = String::new();

        if self.indent > 0 {
            line.push_str("    ".repeat(self.indent).as_str());
        }

        line.push_str(s);
        self.lines.push(line);
    }
}

// resty-cli has a fancier implementation that stores all observed "levels" in
// a hash. Then it iterates over 1..$max_level and checks for hash membership,
// picking the first level that is not found in the hash. It's a lot of work
// just to pick the shortest possible delimiter--something that doesn't really
// matter at all.
//
// Input: ab]=]cd]====]ef
//
// resty-cli:
//   get_bracket_level() -> 2
//   quote_lua_string()  -> [==[ab]=]cd]====]ef]==]
// rusty-cli:
//   get_bracket_level() -> 5
//   quote_lua_string()  -> [=====[ab]=]cd]====]ef]=====]
fn get_bracket_level(s: &str) -> usize {
    let mut max_level = 0;

    let mut level = 0;
    let mut last: char = '_';

    for c in s.chars() {
        match (last, c) {
            (']', '=') => level = 1,
            ('=', '=') => level += 1,
            ('=', ']') => max_level = max(max_level, level),
            (_, _) => level = 0,
        }

        last = c;
    }

    max_level + 1
}

pub(crate) fn quote_lua_string(s: &str) -> String {
    let eq = "=".repeat(get_bracket_level(s));

    format!("[{}[{}]{}]", eq, s, eq)
}

fn insert_lua_file_loader(buf: &mut Buf, fname: &str, inline: bool) {
    buf.append(&format!("local fname = {}", quote_lua_string(fname)));
    buf.append(r#"local f = assert(io.open(fname, "r"))"#);
    buf.append(r#"local chunk = f:read("*a")"#);

    let chunk_name = match inline {
        true => "=(command line -e)".to_string(),
        false => format!("@{}", fname),
    };

    let chunk_type = match inline {
        true => "inline",
        false => "file",
    };

    buf.append(&format!(
        "local {}_gen = assert(loadstring(chunk, {}))",
        chunk_type,
        quote_lua_string(chunk_name.as_str())
    ));
}

fn insert_inline_lua(buf: &mut Buf, prefix: &Prefix, lua: &Vec<String>) {
    buf.append("-- inline lua code");

    if lua.is_empty() {
        buf.append("local inline_gen");
        return;
    }

    let path = prefix.conf.join("a.lua");
    let fname = path.to_str().to_owned().unwrap();

    let mut fh = fs::File::create(&path).unwrap();
    fh.write_all(lua.join("; ").as_bytes()).unwrap();
    fh.flush().unwrap();
    insert_lua_file_loader(buf, fname, true);
}

fn insert_code_for_lua_file(buf: &mut Buf, file: &Option<String>) {
    buf.append("-- lua file");
    if file.is_none() {
        buf.append("local file_gen");
        return;
    }

    let fname = file.clone().unwrap();
    insert_lua_file_loader(buf, fname.as_str(), false);
}

fn insert_lua_args(buf: &mut Buf, file: &Option<String>, args: &Vec<String>) {
    buf.append("-- cli args");
    buf.append("arg = {}");

    buf.append(&format!(
        "arg[0] = {}",
        match file {
            Some(fname) => quote_lua_string(fname.as_str()),
            None => quote_lua_string("./conf/a.lua"),
        }
    ));

    for (i, arg) in args.iter().enumerate() {
        buf.append(&format!(
            "arg[{}] = {}",
            i + 1,
            quote_lua_string(arg).as_str()
        ));
    }

    let lua_args = match file {
        // + 1 because we count the Lua filename
        Some(_) => args.len() + 1,
        _ => 0,
    };

    let prog = env::args().nth(0).unwrap();
    let all_args = env::args().len();

    let pos: i32 = (all_args - lua_args).try_into().unwrap();

    buf.append(&format!(
        "arg[{}] = {}",
        0 - pos,
        quote_lua_string(prog.as_str())
    ));
}

pub(crate) fn generate_lua_loader(
    prefix: &Prefix,
    file: &Option<String>,
    inline: &Vec<String>,
    lua_args: &Vec<String>,
) -> Vec<String> {
    let mut buf = Buf::new();
    buf.append("local gen");
    buf.append("do");
    buf.indent += 1;

    insert_lua_args(&mut buf, file, lua_args);
    buf.newline();

    insert_inline_lua(&mut buf, prefix, inline);
    buf.newline();

    insert_code_for_lua_file(&mut buf, file);
    buf.newline();

    buf.append("gen = function()");
    buf.indent += 1;
    buf.append("if inline_gen then inline_gen() end");
    buf.append("if file_gen then file_gen() end");
    buf.indent -= 1;
    buf.append("end");
    buf.indent -= 1;
    buf.append("end");

    buf.lines
}

pub(crate) fn package_path(dirs: &Vec<String>) -> Option<String> {
    if dirs.is_empty() {
        return None;
    }

    let mut path = String::from("lua_package_path \"");
    for dir in dirs {
        path.push_str(dir);
        path.push_str("/?.ljbc;");

        path.push_str(dir);
        path.push_str("/?.lua;");

        path.push_str(dir);
        path.push_str("/?/init.ljbc;");

        path.push_str(dir);
        path.push_str("/?/init.lua;");
    }

    // extra `;` at the end to ensure the system default path is included
    path.push_str(";\";");
    Some(path)
}

pub(crate) fn package_cpath(dirs: &Vec<String>) -> Option<String> {
    if dirs.is_empty() {
        return None;
    }

    let mut path = String::from("lua_package_cpath \"");
    for dir in dirs {
        path.push_str(dir);
        path.push_str("/?.so;");
    }

    // extra `;` at the end to ensure the system default path is included
    path.push_str(";\";");
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_bracket_level() {
        assert_eq!(1, get_bracket_level(""));
        assert_eq!(1, get_bracket_level("abc]]"));
        assert_eq!(2, get_bracket_level("abc]=]"));
        assert_eq!(3, get_bracket_level("abc]==]"));
        assert_eq!(3, get_bracket_level("abc]==]asdf asdf3]=]]]"));
        assert_eq!(3, get_bracket_level("abc]==]asdf asdf3]==]]]"));
        assert_eq!(4, get_bracket_level("abc]==]asdf asdf3]===]]]"));
    }

    #[test]
    fn test_quote_lua_string() {
        assert_eq!("[=[abc]=]", quote_lua_string("abc"));
        assert_eq!("[=[abc]]def]=]", quote_lua_string("abc]]def"));
        assert_eq!("[==[abc]=]def]==]", quote_lua_string("abc]=]def"));
        assert_eq!("[=[[[abc]=]", quote_lua_string("[[abc"));
        assert_eq!("[=[abc[[[def]=]", quote_lua_string("abc[[[def"));
    }

    #[test]
    fn test_package_path() {
        assert_eq!(None, package_path(&vec![]));
        assert_eq!(
            Some(String::from(
                r#"lua_package_path "/foo/?.ljbc;/foo/?.lua;/foo/?/init.ljbc;/foo/?/init.lua;;";"#
            )),
            package_path(&vec![String::from("/foo")])
        );
    }

    #[test]
    fn test_package_cpath() {
        assert_eq!(None, package_cpath(&vec![]));
        assert_eq!(
            Some(String::from(r#"lua_package_cpath "/foo/?.so;;";"#)),
            package_cpath(&vec![String::from("/foo")])
        );
    }
}
