use libc::ESRCH;
use nix::sys::signal as ns;
use signal_child::signal;
use signal_child::signal::Signal;
use std::convert::TryFrom;
use std::process::{Child, Command};
use std::sync::Arc;
use std::sync::{Condvar, Mutex};
use std::thread;
use std::time::Duration;

use libc::{
    SIGCHLD, SIGHUP, SIGINT, SIGKILL, SIGPIPE, SIGQUIT, SIGTERM, SIGUSR1, SIGUSR2, SIGWINCH,
};

const HANDLED: [i32; 9] = [
    SIGINT, SIGTERM, SIGQUIT, SIGHUP, SIGUSR1, SIGUSR2, SIGWINCH, SIGPIPE, SIGCHLD,
];

const SIG_DFL: nix::sys::signal::SigHandler = nix::sys::signal::SigHandler::SigDfl;

lazy_static! {
    static ref COND: Arc<(Mutex<i32>, Condvar)> = Arc::new((Mutex::new(0), Condvar::new()));
}

fn send_signal(pid: u32, signum: i32) {
    let sig = Signal::try_from(signum).expect("Invalid signal {signum}");

    signal(pid as i32, sig)
        .map_err(|e| {
            match e.raw_os_error() {
                // the child process is already gone
                // https://github.com/openresty/resty-cli/pull/39
                Some(ESRCH) => Ok(()),
                _ => Err(e),
            }
        })
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

fn set_caught_signal(signum: i32) {
    if let Ok(mut lock) = COND.0.try_lock() {
        if *lock == 0 {
            *lock = signum;
            COND.1.notify_all();
        }
        drop(lock);
    }
}

extern "C" fn sig_handler(signum: i32) {
    set_caught_signal(signum);

    unsafe {
        let sig = ns::Signal::try_from(signum).unwrap();
        ns::signal(sig, SIG_DFL).unwrap();
    }
}

pub fn run(mut cmd: Command) -> i32 {
    let (lock, cond) = &*COND.clone();

    for signum in HANDLED {
        let sig = ns::Signal::try_from(signum).unwrap();
        unsafe { ns::signal(sig, ns::SigHandler::Handler(sig_handler)) }.unwrap();
    }

    let proc = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            eprintln!(
                "ERROR: failed to run command {}: {}",
                cmd.get_program().to_str().unwrap(),
                e
            );
            return 2;
        }
    };

    let pid = proc.id();

    let mut caught = lock.lock().unwrap();
    while *caught == 0 {
        caught = cond.wait(caught).unwrap();
    }

    match *caught {
        SIGCHLD => block_wait(proc).unwrap_or(0),
        SIGINT => {
            send_then_kill(pid, SIGQUIT);
            block_wait(proc);
            128 + SIGINT
        }
        SIGPIPE => {
            send_then_kill(pid, SIGQUIT);
            block_wait(proc);
            128 + SIGPIPE
        }
        SIGHUP => {
            send_signal(pid, SIGQUIT);
            block_wait(proc).unwrap_or(0)
        }
        SIGTERM => {
            send_signal(pid, SIGTERM);
            block_wait(proc);
            128 + SIGTERM
        }
        other => {
            send_signal(pid, other);
            block_wait(proc).unwrap_or(0)
        }
    }
}
