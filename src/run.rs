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

type SignalIterator = SignalsInfo::<exfiltrator::WithOrigin>;

fn register_signal_handlers() -> SignalIterator {
    let term = Arc::new(AtomicBool::new(false));

    for sig in HANDLED {
        flag::register_conditional_default(sig, Arc::clone(&term))
            .expect("Failed registering signal handler");
        flag::register(sig, Arc::clone(&term)).expect("Failed registering signal handler");
    }

    SignalsInfo::new(HANDLED).expect("Failed to create signal iterator")
}

fn send_signal(pid: u32, signum: i32) {
    let sig = Signal::try_from(signum)
        .expect("Invalid signal {signum}");

    signal(pid as i32, sig)
        .expect("Failed sending signal to child process");
}

fn send_then_kill(pid: u32, signum: i32) {
    send_signal(pid, signum);
    thread::sleep(Duration::from_millis(100));
    send_signal(pid, SIGKILL);
}

fn block_wait(mut proc: Child) -> Option<i32> {
    proc.wait()
        .expect("Failed waiting for child process to exit")
        .code()
}

pub fn run(mut cmd: Command) -> i32 {
    let mut signals = register_signal_handlers();

    let proc = cmd.spawn().expect("error spawning Nginx");
    let pid = proc.id();

    let caught = signals
        .forever()
        .next()
        .expect("Failed waiting for a signal")
        .signal;

    match caught {
        SIGCHLD => {
            block_wait(proc).unwrap_or(0)
        },
        SIGINT => {
            send_then_kill(pid, SIGQUIT);
            block_wait(proc);
            130
        }
        SIGPIPE => {
            send_then_kill(pid, SIGQUIT);
            block_wait(proc);
            141
        }
        SIGHUP => {
            send_signal(pid, SIGQUIT);
            block_wait(proc).unwrap_or(0)
        }
        SIGTERM => {
            send_signal(pid, SIGTERM);
            block_wait(proc);
            143
        }
        other => {
            send_signal(pid, other);
            block_wait(proc).unwrap_or(0)
        }
    }
}
