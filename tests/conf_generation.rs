mod common;
use common::*;

#[test]
fn test_conf_generation() {
    let (resty, rusty) = run_both(&vec!["./tests/lua/nginx-conf-to-json.lua"]);
    assert_eq!(resty.exit_code, rusty.exit_code);
    similar_asserts::assert_eq!(resty.stderr, rusty.stderr);
    let resty = json_decode(&resty.stdout);
    let rusty = json_decode(&rusty.stdout);
    similar_asserts::assert_serde_eq!(resty, rusty);
}
