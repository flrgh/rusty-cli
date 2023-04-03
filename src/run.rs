use libc::ESRCH;
use signal_child::signal;
use signal_child::signal::Signal;
use signal_hook::consts::*;
use signal_hook::flag;
use signal_hook::iterator::*;
use signal_hook::low_level as ll;
use std::convert::TryFrom;
use std::io::Read;
use std::process::{Child, Command};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const HANDLED: [i32; 9] = [
    SIGINT, SIGTERM, SIGQUIT, SIGHUP, SIGUSR1, SIGUSR2, SIGWINCH, SIGPIPE, SIGCHLD,
];

//type SignalIterator = SignalsInfo::<exfiltrator::WithOrigin>;
type SignalIterator = SignalsInfo<exfiltrator::SignalOnly>;

use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;

fn register_signal_handlers() -> SignalIterator {
    let term = Arc::new(AtomicBool::new(false));

    for sig in HANDLED {
        if sig != SIGPIPE {
            continue;
        }
        flag::register_conditional_default(sig, Arc::clone(&term))
            .expect("Failed registering signal handler");
        flag::register(sig, Arc::clone(&term)).expect("Failed registering signal handler");
    }

    SignalsInfo::new(&HANDLED).expect("Failed to create signal iterator")
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

const SIG_DFL: nix::sys::signal::SigHandler = nix::sys::signal::SigHandler::SigDfl;

use std::sync::atomic::Ordering;
use std::sync::{Condvar, Mutex};
//use std::sync::Arc;
use nix::sys::signal as ns;
use signal_hook_registry::register;
use std::cell::Cell;
use std::sync::Once;

static mut CAUGHT: i32 = 0;

static SIGNALED: Once = Once::new();

lazy_static! {
    static ref COND: Arc<(Mutex<i32>, Condvar)> = Arc::new((Mutex::new(0), Condvar::new()));
}

fn set_caught_signal(signum: i32) {
    //unsafe { CAUGHT = signum };
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
    //SIGNALED.call_once(|| set_caught_signal(signum));

    unsafe {
        //eprintln!("restoring default handler to signal: {}", signum);
        let sig = ns::Signal::try_from(signum).unwrap();
        ns::signal(sig, SIG_DFL).unwrap();
    }
}

pub fn run(mut cmd: Command) -> i32 {
    //let mut handlers = vec![];

    let (lock, cond) = &*COND.clone();

    for signum in HANDLED {
        let sig = ns::Signal::try_from(signum).unwrap();
        unsafe { ns::signal(sig, ns::SigHandler::Handler(sig_handler)) }.unwrap();
        //handlers.push(hdl);
    }

    let mut proc: std::process::Child;

    match cmd.spawn() {
        Ok(child) => {
            proc = child;
        }
        Err(e) => {
            eprintln!(
                "ERROR: failed to run command {}: {}",
                cmd.get_program().to_str().unwrap(),
                e.to_string()
            );
            return 2;
        }
    }

    let pid = proc.id();

    //    eprintln!("SELF: {}, CHILD: {}", std::process::id(), pid);

    //let mut signals = register_signal_handlers();

    // let caught = signals
    //     .forever()
    //     .next()
    //     .expect("Failed waiting for a signal");
    //

    let mut caught = lock.lock().unwrap();
    while *caught == 0 {
        caught = cond.wait(caught).unwrap();
    }
    drop(lock);

    let caught = caught.clone();

    // let caught = loop {
    //     if SIGNALED.is_completed() {
    //         break unsafe { CAUGHT }
    //     } else {
    //         std::thread::sleep(Duration::from_millis(1));
    //     }
    // };
    //

    match caught {
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
