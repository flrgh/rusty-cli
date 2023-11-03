#[macro_use]
extern crate maplit;

mod common;
use common::*;

#[test]
fn conf_generation() {
    let env = Some(hashmap! {
        str!("RUSTY_STRIP_LUA_INDENT") => str!("1"),
    });

    let args = strings(vec!["./tests/lua/nginx-conf-to-json.lua"]);

    assert_same(&args, &env);
}
