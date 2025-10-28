#![allow(static_mut_refs)]

use nix::sys::signal::{
    SIGHUP, SIGINT, SIGPIPE, SIGQUIT, SIGSEGV, SIGTERM, SIGUSR1, SIGUSR2, SIGWINCH,
};
use nix::sys::signal::{SaFlags, SigAction, SigHandler, SigSet, Signal, sigaction};
use std::fs::{File, metadata};
use std::io::{Read, Write};
use std::os::unix::prelude::MetadataExt;
use std::path::PathBuf;
use std::process;
use std::thread::{self, sleep};
use std::time::Duration;

const SIGNALS: [Signal; 9] = [
    SIGINT, SIGTERM, SIGQUIT, SIGHUP, SIGUSR1, SIGUSR2, SIGWINCH, SIGPIPE, SIGSEGV,
];

use std::sync::Arc;
use std::sync::mpsc::{Sender, channel};

static mut SENDER: Option<Arc<Sender<Option<Signal>>>> = None;

#[unsafe(no_mangle)]
extern "C" fn signal_handler(sig: libc::c_int) {
    let Ok(sig) = Signal::try_from(sig) else {
        return;
    };

    let sender = unsafe { &SENDER };
    let Some(sender) = sender else {
        return;
    };

    let _ = sender.send(Some(sig));
}

fn main() {
    let dir = std::env::var_os("WORKDIR").expect("`WORKDIR` env var must be set");

    let dir = PathBuf::from(dir);
    let sig_fname = dir.join("signals");

    let (send, recv) = channel();

    let send = Arc::new(send);

    unsafe {
        SENDER = Some(send.clone());
    };

    for sig in SIGNALS {
        let handler = SigHandler::Handler(signal_handler);
        let mut mask = SigSet::from_iter(SIGNALS);
        mask.remove(sig);
        let flags = SaFlags::empty() | SaFlags::SA_NODEFER;
        let sa = SigAction::new(handler, flags, mask);

        if let Err(e) = unsafe { sigaction(sig, &sa) } {
            eprintln!("sigaction({sig}) => {e}");
            process::exit(1);
        };
    }

    let logger = {
        let fname = sig_fname.clone();
        thread::spawn(move || {
            let mut f = File::create_new(fname).expect("create signal file");
            for sig in recv.iter() {
                let Some(sig) = sig else {
                    return;
                };

                write!(f, "{sig}\n").expect("write failed");
            }
            drop(f);
        })
    };

    let sig_fname = dir.join("signals");
    while !sig_fname.exists() {
        sleep(Duration::from_millis(10));
    }

    {
        let pid_file = dir.join("pid");
        let mut f = File::create_new(pid_file).expect("create pid file");
        write!(f, "{0}", std::process::id()).expect("write pid file");
        drop(f);
    }

    let exit_fname = dir.join("exit");
    while !metadata(&exit_fname).is_ok_and(|st| st.size() > 0) {
        sleep(Duration::from_millis(10));
    }

    let rc = {
        let mut f = File::open(exit_fname).expect("open exit file");
        let mut buf: [u8; 1] = [0; 1];
        f.read(&mut buf).expect("read exit file");

        let c: char = buf[0].into();
        c.to_digit(10).unwrap_or(0)
    };

    let _ = send.send(None);
    logger.join().expect("logging thread join");

    process::exit(rc as i32);
}
