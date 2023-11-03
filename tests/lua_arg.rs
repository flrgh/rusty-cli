#[macro_use]
extern crate maplit;

mod common;
use common::*;


#[test]
fn lua_arg() {
    fn test_one(args: &Vec<String>) {
        let temp = TempDir::new();
        let resty_out = temp
            .clone()
            .join("resty.json")
            .to_str()
            .unwrap()
            .to_string();
        let rusty_out = temp
            .clone()
            .join("rusty.json")
            .to_str()
            .unwrap()
            .to_string();

        let resty = {
            let env = hashmap! {
                str!("RUSTY_CLI_TEST_OUTPUT") => resty_out.clone(),
            };

            run(RESTY, args.clone(), Some(env.clone()))
        };

        let rusty = {
            let env = hashmap! {
                str!("RUSTY_CLI_TEST_OUTPUT") => rusty_out.clone(),
            };

            run(RUSTY, args.clone(), Some(env.clone()))
        };

        similar_asserts::assert_eq!(resty, rusty);

        if resty.exit_code == 0 {
            let resty = std::fs::read_to_string(&resty_out).expect("couldn't read");
            let resty = patch_result(&resty);
            let resty = json_decode(&resty);

            let rusty = std::fs::read_to_string(&rusty_out).expect("couldn't read");
            let rusty = patch_result(&rusty);
            let rusty = json_decode(&rusty);

            similar_asserts::assert_serde_eq!(resty, rusty);
        }
    }

    fn test_all(args: &Vec<String>) {
        let argv_file = "./tests/lua/print-argv.lua".to_string();
        let argv_script = format!("dofile(\"{argv_file}\")");

        let mut file_args = vec![];
        let mut inline_args = vec![];
        let mut inline_eq_args = vec![];

        for arg in args {
            let arg = arg.clone();

            if arg == "LUA_ARGV" {
                file_args.push(argv_file.clone());

                inline_args.push("-e".into());
                inline_args.push(argv_script.clone());

                inline_eq_args.push(format!("-e={argv_script}"));
            } else {
                file_args.push(arg.clone());
                inline_args.push(arg.clone());
                inline_eq_args.push(arg.clone());
            }
        }

        test_one(&file_args);
        test_one(&inline_args);
        test_one(&inline_eq_args);
    }

    test_all(&strings(vec!["LUA_ARGV"]));
    test_all(&strings(vec!["-e", "tostring('BEFORE')", "LUA_ARGV"]));
    test_all(&strings(vec!["LUA_ARGV", "-e", "tostring('AFTER')"]));
    test_all(&strings(vec![
        "-e",
        "tostring('BEFORE')",
        "LUA_ARGV",
        "-e",
        "tostring('AFTER')",
    ]));
    test_all(&strings(vec!["-e=tostring('BEFORE')", "LUA_ARGV"]));

    test_all(&strings(vec![
        "-I",
        "test",
        "-e",
        "tostring(1)",
        "-c",
        "10",
        "LUA_ARGV",
        "--ns",
        "1.2.3.4",
        "-e",
        "tostring(2)",
    ]));

    test_all(&strings(vec!["LUA_ARGV", "a", "b", "c", "d"]));
    test_all(&strings(vec!["LUA_ARGV", "--", "a", "b", "c", "d"]));
}
