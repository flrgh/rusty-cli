use nix::errno::Errno::{self, ESRCH};
use nix::sys::signal::{kill, sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal};
use nix::sys::signal::{
    SIGHUP, SIGINT, SIGKILL, SIGPIPE, SIGQUIT, SIGSEGV, SIGTERM, SIGUSR1, SIGUSR2, SIGWINCH,
};
use nix::unistd::Pid;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::thread;
use std::time::Duration;

const SIGNALS: [Signal; 8] = [
    SIGINT, SIGTERM, SIGQUIT, SIGHUP, SIGUSR1, SIGUSR2, SIGWINCH, SIGPIPE,
];

static HANDLED: AtomicBool = AtomicBool::new(false);
static SIGNAL: AtomicI32 = AtomicI32::new(0);
static CHILD_PID: AtomicI32 = AtomicI32::new(0);

fn send_signal_unchecked(pid: Pid, sig: Signal) {
    let _ = kill(pid, sig);
}

fn send_signal(pid: Pid, sig: Signal) {
    match kill(pid, sig) {
        Err(ESRCH) => {} // child process is already gone
        Err(e) => {
            eprintln!("failed sending signal {sig} to {pid}: {e}");
        }
        Ok(_) => {}
    }
}

fn set_caught_signal(sig: Signal) {
    if !HANDLED.swap(true, Ordering::Acquire) {
        SIGNAL.store(sig as i32, Ordering::Release);
    }
}

fn get_caught_signal() -> Option<i32> {
    let caught = SIGNAL.load(Ordering::Relaxed);
    if caught == 0 {
        None
    } else {
        Some(caught)
    }
}

fn send_quit_and_kill(pid: Pid) {
    send_signal(pid, SIGQUIT);
    thread::sleep(Duration::from_millis(100));
    send_signal(pid, SIGKILL);
}

#[no_mangle]
extern "C" fn signal_handler(sig: libc::c_int) {
    let Ok(sig) = Signal::try_from(sig) else {
        return;
    };

    let pid = CHILD_PID.load(Ordering::Relaxed);
    if pid <= 0 {
        return;
    }
    let pid = Pid::from_raw(pid);

    match sig {
        SIGINT | SIGPIPE => {
            set_caught_signal(sig);
            send_quit_and_kill(pid);
        }

        SIGTERM => {
            set_caught_signal(sig);
            send_signal_unchecked(pid, sig);
        }

        SIGHUP => {
            // translate to QUIT
            send_signal_unchecked(pid, SIGQUIT);
        }

        SIGWINCH | SIGQUIT | SIGUSR1 | SIGUSR2 => {
            // forward as-is
            send_signal_unchecked(pid, sig);
        }
        // ignore others
        _ => {}
    }
}

struct SignalAction(Signal, SigAction);

impl SignalAction {
    fn try_new(signal: Signal) -> Result<Self, Errno> {
        let handler = SigHandler::Handler(signal_handler);
        let mask = SigSet::from_iter(SIGNALS);
        let flags = SaFlags::empty();

        let action = SigAction::new(handler, flags, mask);

        // SAFETY: sigaction() is unsafe largely because the compiler cannot
        // guarantee that the signal handler function is async signal safe.
        // The syscalls our signal handler makes use of (`sleep()`, `kill()`)
        // are marked as safe to use by the `signal-safety(7)` document, and
        // the only global state they touch is accessed via atomics.
        match unsafe { sigaction(signal, &action) } {
            Ok(old) => Ok(SignalAction(signal, old)),
            Err(e) => Err(e),
        }
    }
}

impl Drop for SignalAction {
    fn drop(&mut self) {
        // SAFETY: we are restoring the `sigaction` struct that was given to us
        // by the operating system from our previous `sigaction(2)` call
        let _ = unsafe { sigaction(self.0, &self.1) };
    }
}

pub(crate) fn run(mut cmd: Command) -> i32 {
    let mut old_actions = Vec::with_capacity(SIGNALS.len());
    for signal in SIGNALS {
        match SignalAction::try_new(signal) {
            Ok(action) => old_actions.push(action),
            Err(e) => {
                eprintln!("sigaction({signal}) failure => {e}");
                return 2;
            }
        }
    }

    let mut proc = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            let prog = cmd.get_program().to_string_lossy();
            eprintln!("ERROR: failed to run command \"{prog}\": {e}");
            return 2;
        }
    };

    CHILD_PID.store(proc.id() as i32, Ordering::Relaxed);
    let res = proc.wait();

    // restore signal handlers to their defaults as soon as possible
    drop(old_actions);

    match res {
        Ok(status) => {
            let signal = get_caught_signal().or_else(|| {
                status.signal().or(status.stopped_signal()).or_else(|| {
                    if status.core_dumped() {
                        Some(SIGSEGV as i32)
                    } else {
                        None
                    }
                })
            });

            match (status.code(), signal) {
                (Some(rc), None) => rc,
                (Some(_rc), Some(sig)) => sig + 128,
                (None, Some(sig)) => sig + 128,
                (None, None) => {
                    eprintln!("WARN: nginx exited without a status code but was not signaled");
                    127
                }
            }
        }
        Err(e) => {
            eprintln!("failed waiting for child process to exit: {e}");
            let _ = proc.kill();
            SIGKILL as i32 + 128
        }
    }
}
