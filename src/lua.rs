use crate::types::*;
use std::cmp::max;
use std::env;
use std::fs;
use std::io::Write as IoWrite;

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

pub fn quote_lua_string(s: &str) -> String {
    let eq = "=".repeat(get_bracket_level(s));

    format!("[{}[{}]{}]", eq, s, eq)
}

fn generate_lua_file_loader(fname: &str, inline: bool) -> String {
    let chunk_name = match inline {
        true => "=(command line -e)".to_string(),
        false => format!("@{}", fname),
    };

    let chunk_type = match inline {
        true => "inline",
        false => "file",
    };

    format!(
        r#"
    local fname = {}
    local f = assert(io.open(fname, "r"))
    local chunk = f:read("*a")
    local {}_gen = assert(loadstring(chunk, {}))
"#,
        quote_lua_string(fname),
        chunk_type,
        quote_lua_string(chunk_name.as_str())
    )
}

fn generate_code_for_inline_lua(prefix: &Prefix, lua: &Vec<String>) -> String {
    if lua.is_empty() {
        return "local inline_gen\n".to_string();
    }

    let path = prefix.conf.join("a.lua");
    let fname = path.to_str().to_owned().unwrap();

    let mut fh = fs::File::create(&path).unwrap();
    fh.write_all(lua.join("; ").as_bytes()).unwrap();
    fh.flush().unwrap();
    generate_lua_file_loader(fname, true)
}

fn generate_code_for_lua_file(file: &Option<String>) -> String {
    if file.is_none() {
        return "local file_gen\n".to_string();
    }

    let fname = file.clone().unwrap();
    generate_lua_file_loader(fname.as_str(), false)
}

fn generate_lua_args(file: &Option<String>, args: &Vec<String>) -> String {
    let mut code = String::from("arg = {}\n");

    code.push_str("    arg[0] = ");
    let fname = match file {
        Some(fname) => quote_lua_string(fname.as_str()),
        None => quote_lua_string("./conf/a.lua"),
    };
    code.push_str(fname.as_str());
    code.push_str("\n");

    for (i, arg) in args.iter().enumerate() {
        code.push_str(format!("    arg[{}] = {}\n", i + 1, quote_lua_string(arg)).as_str());
    }

    let lua_args = match file {
        // + 1 because we count the Lua filename
        Some(_) => args.len() + 1,
        _ => 0,
    };

    let prog = env::args().nth(0).unwrap();
    let all_args = env::args().len();

    let pos: i16 = (all_args - lua_args).try_into().unwrap();

    code.push_str(
        format!(
            "    arg[{}] = {}\n",
            0 - pos,
            quote_lua_string(prog.as_str())
        )
        .as_str(),
    );

    code
}

pub fn generate_lua_loader(
    prefix: &Prefix,
    file: &Option<String>,
    inline: &Vec<String>,
    lua_args: &Vec<String>,
) -> String {
    format!(
        r#"
local gen
do
    -- cli args
    {}

    -- inline lua code (-e)
    {}

    -- lua file
    {}

    gen = function()
      if inline_gen then inline_gen() end
      if file_gen then file_gen() end
    end
end"#,
        generate_lua_args(file, lua_args),
        generate_code_for_inline_lua(prefix, inline),
        generate_code_for_lua_file(file)
    )
}

pub fn package_path(dirs: &Vec<String>) -> Option<String> {
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

pub fn package_cpath(dirs: &Vec<String>) -> Option<String> {
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
