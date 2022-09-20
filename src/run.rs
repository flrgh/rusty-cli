use signal_child::signal;
use signal_child::signal::Signal;
use signal_hook::consts::*;
use signal_hook::flag;
use signal_hook::iterator::*;
use std::convert::TryFrom;
use std::process::{Child, Command};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const HANDLED: [i32; 9] = [
    SIGINT, SIGTERM, SIGQUIT, SIGHUP, SIGUSR1, SIGUSR2, SIGWINCH, SIGPIPE, SIGCHLD,
];

fn register_signal_handlers() {
    let term = Arc::new(AtomicBool::new(false));

    for sig in HANDLED {
        flag::register_conditional_default(sig, Arc::clone(&term))
            .expect("Failed registering signal handler");
        flag::register(sig, Arc::clone(&term)).expect("Failed registering signal handler");
    }
}

fn wait_signal() -> i32 {
    let mut signals = SignalsInfo::<exfiltrator::WithOrigin>::new(&HANDLED).unwrap();

    loop {
        if let Some(s) = signals.pending().next() {
            return s.signal;
        }

        thread::sleep(Duration::from_millis(1));
    }
}

fn send_signal(pid: u32, signum: i32) {
    let sig = Signal::try_from(signum).expect("Invalid signal {signum}");
    signal(pid as i32, sig).expect("Failed sending signal to child process");
}

fn send_then_kill(pid: u32, signum: i32) {
    send_signal(pid, signum);
    thread::sleep(Duration::from_millis(100));
    send_signal(pid, SIGKILL);
}

fn block_wait(mut proc: Child) -> Option<i32> {
    let result = proc
        .wait()
        .expect("Failed waiting for child process to exit");
    result.code()
}

fn wait_timeout(mut proc: Child) -> Option<i32> {
    let mut nap = Duration::from_micros(1_000);
    let mut slept = Duration::from_micros(0);
    let timeout = Duration::from_micros(100_000);

    let mut code = None;

    while slept < timeout {
        nap = nap + (nap / 2);
        //eprintln!("current nap {:?}", nap / 1000);
        slept += nap;
        thread::sleep(nap);

        match proc.try_wait() {
            Ok(Some(status)) => {
                code = status.code();
                break;
            }
            Ok(None) => {
                continue;
            }
            Err(e) => {
                eprintln!("OH NO: {e}");
                unreachable!();
            }
        }
    }

    if code.unwrap_or(0) == 0 {
        return None;
    }
    code
}

pub fn run(mut cmd: Command) -> i32 {
    register_signal_handlers();

    let proc = cmd.spawn().expect("error spawning Nginx");
    let pid = proc.id();

    match wait_signal() {
        SIGCHLD => block_wait(proc).unwrap_or(0),
        SIGINT => {
            send_then_kill(pid, SIGQUIT);
            130
        }
        SIGPIPE => {
            send_then_kill(pid, SIGQUIT);
            141
        }
        SIGHUP => {
            send_signal(pid, SIGQUIT);
            wait_timeout(proc).unwrap_or(0)
        }
        SIGTERM => {
            send_signal(pid, SIGTERM);
            wait_timeout(proc).unwrap_or(143)
        }
        other => {
            send_signal(pid, other);
            wait_timeout(proc).unwrap_or(0)
        }
    }
}
