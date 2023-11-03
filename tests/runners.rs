#[macro_use]
extern crate maplit;

mod common;
use common::*;

#[test]
fn runners() {
    let mut path = String::from("./tests/runners/bin");
    if let Ok(env_path) = std::env::var("PATH") {
        path.push(':');
        path.push_str(&env_path);
    };

    let mut env = hashmap! {
        str!("PATH") => path,
    };

    let version: String;
    if let Ok(v) = std::env::var("RESTY_CLI_COMPAT_VERSION") {
        version = v;
        env.insert(str!("RESTY_CLI_COMPAT_VERSION"), version);
    };

    let vars = &Some(env);

    let runner_opts = "-a --flag --option \"my quoted $value\" --foo=bar a b c";

    let user_runner_with_opts = format!("custom-user-runner {runner_opts}");

    let tests = vec![
        // default/no custom runner
        vec![],
        // gdb
        vec!["--gdb"],
        vec!["--gdb", "--gdb-opts", runner_opts],
        // valgrind
        vec!["--valgrind"],
        vec!["--valgrind", "--valgrind-opts", runner_opts],
        // stap
        vec!["--stap"],
        vec!["--stap", "--stap-opts", runner_opts],
        // user
        vec!["--user-runner", "custom-user-runner"],
        vec!["--user-runner", user_runner_with_opts.as_ref()],
        // rr
        vec!["--rr"],
    ];

    let common_args = strings(vec![
        "-e",
        "print(\"hello\")",
        "--errlog-level=notice",
        "-I",
        "./path/to/directory",
        "-e",
        "print(\", world!\")",
    ]);

    for case in tests {
        let mut args = strings(case);
        args.append(&mut common_args.clone());

        assert_same(&args, vars);
    }
}
