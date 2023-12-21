use crate::types::*;
use std::cmp::max;
use std::fs;

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

pub(crate) trait LuaString {
    fn lua_quote(&self) -> String;
}

impl<T> LuaString for T
where
    T: AsRef<str>,
{
    fn lua_quote(&self) -> String {
        quote_lua_string(self.as_ref())
    }
}

pub(crate) fn generate_lua_loader(
    prefix: &Prefix,
    file: &Option<String>,
    inline: &Vec<String>,
    lua_args: &Vec<String>,
    arg_0: String,
    all_args_len: usize,
) -> Result<Vec<String>, std::io::Error> {
    let buf = Buf::new();
    let inline_filename = prefix.conf.join("a.lua").to_str().unwrap().to_owned();

    LuaGenerator {
        arg_0,
        all_args_len,
        file,
        inline,
        lua_args,
        buf,
        inline_filename,
    }
    .generate()
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

#[derive(Debug)]
struct LuaGenerator<'a> {
    file: &'a Option<String>,
    inline: &'a Vec<String>,
    lua_args: &'a Vec<String>,
    inline_filename: String,
    buf: Buf,
    arg_0: String,
    all_args_len: usize,
}

impl<'a> LuaGenerator<'a> {
    pub(crate) fn generate(mut self) -> Result<Vec<String>, std::io::Error> {
        self.buf.append("local gen");
        self.buf.append("do");
        self.buf.indent();

        self.insert_lua_args();
        self.buf.newline();

        self.insert_inline_lua()?;
        self.buf.newline();

        self.insert_code_for_lua_file();
        self.buf.newline();

        self.buf.append("gen = function()");
        self.buf.indent();
        self.buf.append("if inline_gen then inline_gen() end");
        self.buf.append("if file_gen then file_gen() end");
        self.buf.dedent();
        self.buf.append("end");

        self.buf.dedent();
        self.buf.append("end");

        Ok(self.buf.finalize())
    }

    fn insert_lua_args(&mut self) {
        self.buf.append("arg = {}");

        self.buf.append(&format!(
            "arg[0] = {}",
            match self.file {
                Some(fname) => fname,
                None => &self.inline_filename,
            }
            .lua_quote()
        ));

        for (i, arg) in self.lua_args.iter().enumerate() {
            self.buf
                .append(&format!("arg[{}] = {}", i + 1, arg.lua_quote().as_str()));
        }

        let mut lua_args_len = self.lua_args.len() as i32;
        if self.file.is_some() {
            lua_args_len += 1;
        }

        let prog = self.arg_0.lua_quote();

        let mut all_args_len = self.all_args_len as i32;

        if self.file.is_none() {
            all_args_len -= 1;
        }

        let pos = all_args_len - lua_args_len;

        self.buf.append(&format!("arg[{}] = {}", 0 - pos, prog));
    }

    fn insert_inline_lua(&mut self) -> Result<(), std::io::Error> {
        self.buf.append("-- inline lua code");

        if self.inline.is_empty() {
            return Ok(());
        }

        self.buf.append("local inline_gen");

        let contents = self.inline.join("; ");
        let fname = self.inline_filename.clone();
        fs::write(&fname, contents)?;

        self.insert_lua_file_loader(&fname, true);

        Ok(())
    }

    fn insert_code_for_lua_file(&mut self) {
        self.buf.append("-- lua file");
        if self.file.is_none() {
            self.buf.append("local file_gen");
            return;
        }

        let fname = self.file.clone().unwrap();
        self.insert_lua_file_loader(fname.as_str(), false);
    }

    fn insert_lua_file_loader(&mut self, fname: &str, inline: bool) {
        self.buf
            .append(&format!("local fname = {}", fname.lua_quote()));
        self.buf.append(r#"local f = assert(io.open(fname, "r"))"#);
        self.buf.append(r#"local chunk = f:read("*a")"#);

        let chunk_name = match inline {
            true => "=(command line -e)".to_string(),
            false => format!("@{}", fname),
        };

        let chunk_type = match inline {
            true => "inline",
            false => "file",
        };

        self.buf.append(&format!(
            "local {}_gen = assert(loadstring(chunk, {}))",
            chunk_type,
            chunk_name.lua_quote()
        ));
    }
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
