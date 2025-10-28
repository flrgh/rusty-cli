mod testlib;
use testlib::*;

use testlib::sigscript::Script;

type SigRes = (i32, i32, Vec<Signal>);

fn send_signal(pid: Pid, sig: Signal) {
    kill(pid, sig).expect("kill()");
}

fn sigtest(bin: testlib::Bin, run: fn(bin: testlib::Bin, ppid: Pid, pid: Pid) -> SigRes) {
    let tmp = testlib::tmpdir();
    let sig_fname = tmp.join("signals");
    let pid_fname = tmp.join("pid");
    let exit_fname = tmp.join("exit");

    let mut cmd = bin.cmd();
    let nginx = testlib::testbin("signal_logger");
    cmd.env("WORKDIR", tmp.path());
    cmd.args(["--nginx", nginx.as_str(), "-e", "return 'nothing'"]);

    let mut proc = cmd.spawn().expect("command spawned");
    let cleanup_parent = testlib::cleanup_proc(&proc);
    let ppid = Pid::from_raw(proc.id() as i32);

    //let pid = testlib::get_pid(proc.stdout.as_mut().expect("stdout"));

    let pid = testlib::wait_pidfile(pid_fname);
    let cleanup_child = testlib::cleanup_proc(pid);

    // resty-cli startup is sometimes slow, so give it some extra time to set
    // up its signal handlers
    testlib::sleep_ms(250);

    let (exit_with, exp_ec, exp_sigs) = run(bin, ppid, pid);

    testlib::sleep_ms(250);
    touch!(exit_fname, exit_with);

    let res = proc.wait().expect("proc.wait() for CLI to exit");
    let got_ec = res.code().expect("proc.wait() status code");

    assert!(
        !cleanup_parent.exists(),
        "resty/rusty process is still alive"
    );

    assert!(
        !cleanup_child.exists(),
        "child (nginx) process outlived parent"
    );

    assert_eq!(exp_ec, got_ec, "status code $expected != $received",);

    let f = File::open(sig_fname).expect("open signal file");
    let reader = std::io::BufReader::new(f);
    let mut got_sigs = Vec::new();
    for line in reader.lines() {
        let sig = line.expect("line");
        got_sigs.push(Signal::from_str(&sig).expect("parsing signal from string"));
    }

    assert_eq!(exp_sigs, got_sigs, "signals $expected != $received",);
}

enum ExpectedSignals {
    None,
    AtLeast(usize, Signal),
    Exactly(Vec<Signal>),
}

impl Default for ExpectedSignals {
    fn default() -> Self {
        Self::None
    }
}

impl ExpectedSignals {
    fn assert(self, got: Vec<Signal>) {
        match self {
            Self::None => {
                assert_eq!(0, got.len(), "expected no received signals but got {got:?}");
            }
            Self::AtLeast(exp_count, exp_sig) => {
                let mut count = 0;
                for sig in got {
                    assert_eq!(
                        exp_sig, sig,
                        "expected >= {exp_count} signals but got {sig}"
                    );
                    count += 1;
                }

                assert!(
                    count >= exp_count,
                    "expected >= {exp_count} signals but got {count}"
                );
            }
            Self::Exactly(exp) => {
                assert_eq!(*exp, got, "$expected != $received");
            }
        }
    }
}

impl ExpectedSignals {
    fn at_least(&mut self, count: usize, exp: Signal) {
        *self = Self::AtLeast(count, exp);
    }

    fn exactly(&mut self, exp: &[Signal]) {
        *self = Self::Exactly(Vec::from(exp));
    }
}

fn script(
    bin: testlib::Bin,
    run: fn(bin: testlib::Bin, script: &mut Script, ec: &mut i32, sigs: &mut ExpectedSignals),
) {
    let mut script = Script::default();

    let mut exp_ec = 0;
    let mut exp_sigs = ExpectedSignals::None;
    run(bin, &mut script, &mut exp_ec, &mut exp_sigs);

    let mut cmd = bin.cmd();
    let nginx = testbin("signal_script");
    cmd.env("ACTIONS", script.to_string());
    cmd.args(["--nginx", nginx.as_str(), "-e", "return 'nothing'"]);

    let res = cmd.output().expect("cmd.output()");
    let got_ec = res.status.code().expect("status code");
    assert_eq!(exp_ec, got_ec, "status code $expected != $received");

    let lines = testlib::lines(res.stdout);
    let got_sigs: Vec<Signal> = lines
        .iter()
        .filter_map(|line| {
            if line.is_empty() {
                None
            } else {
                Some(Signal::from_str(line).expect("parsing signal"))
            }
        })
        .collect();

    exp_sigs.assert(got_sigs);
}

#[integration]
mod signals {
    use super::*;

    #[bin_test]
    fn sigint(bin: testlib::Bin) {
        sigtest(bin, |_bin, ppid, _pid| {
            send_signal(ppid, SIGINT);
            (0, 130, vec![SIGQUIT])
        });
    }

    #[bin_test]
    fn sighup(bin: testlib::Bin) {
        sigtest(bin, |_bin, ppid, _pid| {
            send_signal(ppid, SIGHUP);
            (0, 0, vec![SIGQUIT])
        });
    }

    #[bin_test]
    fn sigterm(bin: testlib::Bin) {
        sigtest(bin, |bin, ppid, _pid| {
            send_signal(ppid, SIGTERM);
            if bin.is_resty() {
                (0, 0, vec![SIGTERM])
            } else {
                (0, 143, vec![SIGTERM])
            }
        });
    }

    #[bin_test]
    fn sigquit(bin: testlib::Bin) {
        sigtest(bin, |_bin, ppid, _pid| {
            send_signal(ppid, SIGQUIT);
            (0, 0, vec![SIGQUIT])
        });
    }

    #[bin_test]
    fn sigusr1(bin: testlib::Bin) {
        sigtest(bin, |_bin, ppid, _pid| {
            send_signal(ppid, SIGUSR1);
            (0, 0, vec![SIGUSR1])
        });
    }

    #[bin_test]
    fn sigusr2(bin: testlib::Bin) {
        sigtest(bin, |_bin, ppid, _pid| {
            send_signal(ppid, SIGUSR2);
            (0, 0, vec![SIGUSR2])
        });
    }

    #[bin_test]
    fn sigwinch(bin: testlib::Bin) {
        sigtest(bin, |_bin, ppid, _pid| {
            send_signal(ppid, SIGWINCH);

            testlib::sleep_ms(1000);
            send_signal(ppid, SIGWINCH);

            testlib::sleep_ms(1000);
            send_signal(ppid, SIGWINCH);

            (0, 0, vec![SIGWINCH, SIGWINCH, SIGWINCH])
        });
    }

    #[bin_test]
    fn sigpipe(bin: testlib::Bin) {
        sigtest(bin, |_bin, ppid, _pid| {
            send_signal(ppid, SIGPIPE);
            (0, 141, vec![SIGQUIT])
        });
    }

    #[bin_test]
    fn sigwinch_repeat(bin: testlib::Bin) {
        sigtest(bin, |bin, ppid, _pid| {
            let send = [SIGWINCH; 10];
            for _ in send {
                send_signal(ppid, SIGWINCH);
                testlib::sleep_ms(1);
            }

            // FIXME
            if bin.is_resty() {
                (0, 0, vec![SIGWINCH, SIGWINCH])
            } else {
                (0, 0, Vec::from(send))
            }
        });
    }

    #[bin_test]
    fn panic(bin: testlib::Bin) {
        script(bin, |_bin, script, ec, _sigs| {
            script.log_all(true);
            script.sleep(100);
            script.panic();
            *ec = 101;
        });
    }

    #[bin_test]
    fn nonzero_exit(bin: testlib::Bin) {
        script(bin, |_bin, script, ec, _sigs| {
            script.log_all(true);
            script.exit(111);
            *ec = 111;
        });
    }

    #[bin_test]
    fn segfault(bin: testlib::Bin) {
        script(bin, |_bin, script, ec, _sigs| {
            script.log_all(true);
            script.segfault();
            *ec = 139;
        });
    }

    // tests from resty-cli/t/resty/signals.t
    mod prove {
        use super::*;

        // === TEST 1: Forward SIGINT to child process
        #[bin_test]
        fn sigint(bin: testlib::Bin) {
            script(bin, |_bin, script, ec, sigs| {
                script.log_all(true);

                for _ in 0..3 {
                    script.send(SIGINT);
                    script.sleep(1000);
                }

                *ec = 130;
                sigs.exactly(&[SIGQUIT]);
            });
        }

        // === TEST 2: Convert SIGHUP to SIGQUIT to child process
        #[bin_test]
        fn sighup(bin: testlib::Bin) {
            script(bin, |_bin, script, ec, sigs| {
                script.log_all(true);

                for _ in 0..3 {
                    script.send(SIGHUP).sleep(1000);
                }

                *ec = 0;
                sigs.exactly(&[SIGQUIT]);
            });
        }

        // === TEST 3: Forward SIGTERM to child process
        #[bin_test]
        fn sigterm(bin: testlib::Bin) {
            script(bin, |_bin, script, ec, sigs| {
                script.log_all(true);

                for _ in 0..3 {
                    script.send(SIGTERM).sleep(1000);
                }

                *ec = 143;
                sigs.exactly(&[SIGTERM]);
            });
        }

        // === TEST 4: Forward SIGQUIT to child process
        #[bin_test]
        fn sigquit(bin: testlib::Bin) {
            script(bin, |_bin, script, ec, sigs| {
                script.log_all(true);

                for _ in 0..3 {
                    script.send(SIGQUIT).sleep(1000);
                }

                *ec = 0;
                sigs.exactly(&[SIGQUIT]);
            });
        }

        // === TEST 5: Forward SIGUSR1 to child process
        #[bin_test]
        fn sigusr1(bin: testlib::Bin) {
            script(bin, |_bin, script, ec, sigs| {
                script.log_all(true);

                for _ in 0..3 {
                    script.send(SIGUSR1).sleep(1000);
                }

                *ec = 0;
                sigs.exactly(&[SIGUSR1]);
            });
        }

        // === TEST 6: Forward SIGUSR2 to child process
        #[bin_test]
        fn sigusr2(bin: testlib::Bin) {
            script(bin, |_bin, script, ec, sigs| {
                script.log_all(true);

                for _ in 0..3 {
                    script.send(SIGUSR2).sleep(1000);
                }

                *ec = 0;
                sigs.exactly(&[SIGUSR2]);
            });
        }

        // NOTE: this one is mislabeled as SIGUSR2 in t/resty/signals.t
        // === TEST 7: Forward SIGWINCH to child process
        #[bin_test]
        fn sigwinch(bin: testlib::Bin) {
            script(bin, |_bin, script, ec, sigs| {
                script.log_all(true);

                for _ in 0..3 {
                    script.send(SIGWINCH).sleep(1000);
                }

                *ec = 0;
                sigs.exactly(&[SIGWINCH]);
            });
        }

        // === TEST 8: Convert SIGPIPE to SIGQUIT to child process
        #[bin_test]
        fn sigpipe(bin: testlib::Bin) {
            script(bin, |_bin, script, ec, sigs| {
                script.log_all(true);

                for _ in 0..3 {
                    script.send(SIGPIPE).sleep(1000);
                }

                *ec = 141;
                sigs.exactly(&[SIGQUIT]);
            });
        }

        // === TEST 9: Rapidly send SIGWINCH while child is exiting
        #[bin_test]
        fn sigwinch_repeat(bin: testlib::Bin) {
            script(bin, |bin, script, _ec, sigs| {
                script.log_all(false);

                let send = [SIGWINCH; 10];
                for _ in send {
                    script.send(SIGWINCH);
                    script.sleep(1);
                }

                if bin.is_resty() {
                    sigs.at_least(1, SIGWINCH);
                } else {
                    sigs.at_least(5, SIGWINCH);
                }
            });
        }
    }
}
