use libc::{c_int, kill, pid_t, ESRCH};
use nix::sys::signal::{SigSet, SigmaskHow::SIG_BLOCK, Signal};
use nix::sys::signal::{
    SIGCHLD, SIGHUP, SIGINT, SIGKILL, SIGPIPE, SIGQUIT, SIGTERM, SIGUSR1, SIGUSR2, SIGWINCH,
};
use std::io::Error;
use std::process::{Child, Command};
use std::thread;
use std::time::Duration;

const SIGNALS: [Signal; 9] = [
    SIGINT, SIGTERM, SIGQUIT, SIGHUP, SIGUSR1, SIGUSR2, SIGWINCH, SIGPIPE, SIGCHLD,
];

fn send_signal(proc: &Child, sig: Signal) {
    let pid = proc.id() as pid_t;
    let ret = unsafe { kill(pid, sig as c_int) };

    if ret < 0 {
        let e = Error::last_os_error();
        match e.raw_os_error() {
            None => {}
            Some(ESRCH) => {} // child process is already gone
            Some(_) => {
                eprintln!("failed sending signal to {pid}: {e}");
            }
        }
    }
}

fn send_then_kill(proc: &Child, sig: Signal) {
    send_signal(proc, sig);
    thread::sleep(Duration::from_millis(100));
    send_signal(proc, SIGKILL);
}

fn block_wait(mut proc: Child) -> i32 {
    match proc.wait() {
        Ok(status) => status.code().unwrap_or(0),
        Err(e) => {
            eprintln!("failed waiting for child process to exit: {e}");
            send_signal(&proc, SIGKILL);
            128 + SIGKILL as i32
        }
    }
}

pub fn run(mut cmd: Command) -> i32 {
    let proc = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            let prog = cmd.get_program().to_str().unwrap();
            eprintln!("ERROR: failed to run command {prog}: {e}");
            return 2;
        }
    };

    let caught = {
        let mask = SigSet::from_iter(SIGNALS);
        let old_mask = mask
            .thread_swap_mask(SIG_BLOCK)
            .expect("failed to block signals");

        let signal = mask.wait().expect("failed to wait for signal");

        old_mask
            .thread_set_mask()
            .expect("failed to restore thread signal mask");

        signal
    };

    match caught {
        SIGCHLD => block_wait(proc),
        SIGINT => {
            send_then_kill(&proc, SIGQUIT);
            block_wait(proc);
            128 + SIGINT as i32
        }
        SIGPIPE => {
            send_then_kill(&proc, SIGQUIT);
            block_wait(proc);
            128 + SIGPIPE as i32
        }
        SIGHUP => {
            send_signal(&proc, SIGQUIT);
            block_wait(proc)
        }
        SIGTERM => {
            send_signal(&proc, SIGTERM);
            block_wait(proc);
            128 + SIGTERM as i32
        }
        other => {
            send_signal(&proc, other);
            block_wait(proc)
        }
    }
}
