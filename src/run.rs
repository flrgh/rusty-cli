use libc::pid_t;
use nix::errno::Errno::ESRCH;
use nix::sys::signal::{kill, SigSet, SigmaskHow::SIG_BLOCK, Signal};
use nix::sys::signal::{
    SIGCHLD, SIGHUP, SIGINT, SIGKILL, SIGPIPE, SIGQUIT, SIGTERM, SIGUSR1, SIGUSR2, SIGWINCH,
};
use nix::unistd::Pid;
use std::process::{Child, Command};
use std::thread;
use std::time::Duration;

const SIGNALS: [Signal; 9] = [
    SIGINT, SIGTERM, SIGQUIT, SIGHUP, SIGUSR1, SIGUSR2, SIGWINCH, SIGPIPE, SIGCHLD,
];

fn send_signal(proc: &Child, sig: Signal) {
    let pid = Pid::from_raw(proc.id() as pid_t);
    match kill(pid, sig) {
        Err(ESRCH) => {} // child process is already gone
        Err(e) => {
            eprintln!("failed sending signal {sig} to {pid}: {e}");
        }
        Ok(_) => {}
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

pub(crate) fn run(mut cmd: Command) -> i32 {
    let mask = SigSet::from_iter(SIGNALS);

    let mut proc = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            let prog = cmd.get_program().to_string_lossy();
            eprintln!("ERROR: failed to run command \"{prog}\": {e}");
            return 2;
        }
    };

    let old_mask = mask
        .thread_swap_mask(SIG_BLOCK)
        .expect("failed to block signals");

    let caught = loop {
        // we need to check if the child process has exited some time between
        // when we set up our signal mask and now
        if let Ok(Some(_)) = proc.try_wait() {
            break SIGCHLD;
        }

        let signal = mask.wait().expect("failed to wait for signal");

        if signal == SIGWINCH {
            send_signal(&proc, SIGWINCH);
        } else {
            break signal;
        }
    };

    old_mask
        .thread_set_mask()
        .expect("failed to restore thread signal mask");

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
