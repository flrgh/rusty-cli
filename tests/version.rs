mod testlib;
use testlib::*;

#[integration]
mod version {
    use super::*;

    #[test]
    fn version_command() {
        let nginx = testlib::testbin("print_args");

        let mut cmd = testlib::RUSTY.cmd();
        cmd.env("OUTPUT", "stderr");
        cmd.args(["--nginx", nginx.as_str(), "-v"]);

        assert_empty!(cmd.stdout_lines());

        assert_eq!(
            vec![
                format!("rusty-cli {}", env!("CARGO_PKG_VERSION")),
                format!("ARG[0] {}", nginx.as_str()),
                "ARG[1] -V".into(),
            ],
            cmd.stderr_lines()
        );
    }

    #[test]
    fn version_command_respects_arg0() {
        let nginx = testlib::testbin("print_args");

        let tmp = testlib::tmpdir();
        let exe = tmp.join("my-special-command");
        exe.symlink_to(testlib::RUSTY.path());

        let mut cmd = exe.cmd();
        cmd.env("OUTPUT", "stderr");
        cmd.args(["--nginx", nginx.as_str(), "-v"]);

        assert_empty!(cmd.stdout_lines());

        assert_eq!(
            vec![
                format!("my-special-command {}", env!("CARGO_PKG_VERSION")),
                format!("ARG[0] {}", nginx.as_str()),
                "ARG[1] -V".into(),
            ],
            cmd.stderr_lines()
        );
    }

    #[test]
    fn version_command_resty_arg0() {
        let nginx = testlib::testbin("print_args");

        let tmp = testlib::tmpdir();
        let exe = tmp.join("resty");
        exe.symlink_to(testlib::RUSTY.path());

        let mut cmd = exe.cmd();
        cmd.env("OUTPUT", "stderr");
        cmd.env("RESTY_CLI_COMPAT_VERSION", "v0.29");
        cmd.args(["--nginx", nginx.as_str(), "-v"]);

        assert_empty!(cmd.stdout_lines());

        assert_eq!(
            vec![
                "resty 0.29".into(),
                format!("ARG[0] {}", nginx.as_str()),
                "ARG[1] -V".into(),
            ],
            cmd.stderr_lines()
        );
    }
}
