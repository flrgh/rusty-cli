use nix::sys::signal::{kill, sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal};
use nix::unistd::Pid;
use std::fmt::Display;
use std::process;
use std::str::FromStr;
use std::{thread::sleep, time::Duration};

#[unsafe(no_mangle)]
extern "C" fn log_signal(sig: libc::c_int) {
    match Signal::try_from(sig) {
        Ok(sig) => {
            println!("{sig}");
        }
        Err(_) => {
            println!("{sig}");
        }
    };
}

#[unsafe(no_mangle)]
extern "C" fn log_signal_and_exit(sig: libc::c_int) {
    log_signal(sig);
    process::exit(0);
}

fn split(elem: &str) -> (&str, impl Iterator<Item = &str>) {
    match elem.split_once('=') {
        Some((act, args)) => (act, args.split(',')),
        None => (elem, "".split(',')),
    }
}

pub enum Setup {
    Log(Signal, bool),
    ExitCode(i32),
}

impl Display for Setup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Setup::Log(signal, exit) => {
                write!(f, "log={signal},{exit}")
            }
            Setup::ExitCode(ec) => {
                write!(f, "exit={ec}")
            }
        }
    }
}

impl Setup {
    pub fn exec(self, rc: &mut i32) {
        match self {
            Setup::Log(sig, exit) => {
                let handler = SigHandler::Handler(if exit {
                    log_signal_and_exit
                } else {
                    log_signal
                });

                let mask = SigSet::empty();
                let flags = SaFlags::empty() | SaFlags::SA_NODEFER;
                let sa = SigAction::new(handler, flags, mask);

                unsafe { sigaction(sig, &sa) }.expect("sigaction()");
            }
            Setup::ExitCode(code) => {
                *rc = code;
            }
        }
    }
}

impl FromStr for Setup {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (act, mut args) = split(s);
        match act {
            "log" => {
                let sig = args
                    .next()
                    .and_then(|sig| Signal::from_str(sig).ok())
                    .expect("signal name required");

                let exit = args
                    .next()
                    .and_then(|e| e.parse::<bool>().ok())
                    .unwrap_or(false);

                Ok(Self::Log(sig, exit))
            }

            "exit" => {
                let rc = args
                    .next()
                    .and_then(|rc| rc.parse::<i32>().ok())
                    .expect("exit code required");

                Ok(Self::ExitCode(rc))
            }

            _ => Err(()),
        }
    }
}

pub enum Op {
    Sleep(Duration),
    Signal(Signal),
    Panic,
    SegFault,
}

impl Op {
    pub fn exec(self, pid: Pid) {
        match self {
            Op::Sleep(ms) => sleep(ms),
            Op::Signal(signal) => {
                kill(pid, signal).expect("kill()");
            }
            Op::Panic => panic!("I'm panicked!"),
            Op::SegFault => {
                // SAFETY: this is test code
                unsafe {
                    std::ptr::null_mut::<i32>().write(42);
                }
            }
        }
    }
}

impl Display for Op {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Op::Sleep(duration) => {
                write!(f, "sleep={}", duration.as_millis())
            }
            Op::Signal(signal) => {
                write!(f, "send={signal}")
            }
            Op::Panic => {
                write!(f, "panic")
            }
            Op::SegFault => {
                write!(f, "segfault")
            }
        }
    }
}

impl FromStr for Op {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (op, mut args) = split(s);
        match op {
            "sleep" => args
                .next()
                .and_then(|ms| ms.parse::<u64>().ok())
                .map(|ms| Self::Sleep(Duration::from_millis(ms)))
                .ok_or(()),

            "send" => args
                .next()
                .and_then(|sig| Signal::from_str(sig).ok())
                .map(Self::Signal)
                .ok_or(()),

            "panic" => Ok(Self::Panic),
            "segfault" => Ok(Self::SegFault),
            _ => Err(()),
        }
    }
}

#[derive(Default)]
pub struct Script {
    setups: Vec<Setup>,
    ops: Vec<Op>,
}

static SIGNALS: &[Signal] = &[
    Signal::SIGHUP,
    Signal::SIGINT,
    Signal::SIGPIPE,
    Signal::SIGQUIT,
    Signal::SIGTERM,
    Signal::SIGUSR1,
    Signal::SIGUSR2,
    Signal::SIGWINCH,
];

impl Script {
    pub fn log_all(&mut self, exit: bool) -> &mut Self {
        for sig in SIGNALS.iter() {
            self.log(*sig, exit);
        }
        self
    }

    pub fn log_except(&mut self, s: Signal) -> &mut Self {
        for sig in SIGNALS.iter() {
            if sig == &s {
                continue;
            }
            self.log(*sig, true);
        }
        self
    }

    pub fn log(&mut self, s: Signal, exit: bool) -> &mut Self {
        self.setups.push(Setup::Log(s, exit));
        self
    }

    pub fn exit(&mut self, rc: i32) -> &mut Self {
        self.setups.push(Setup::ExitCode(rc));
        self
    }

    pub fn sleep(&mut self, ms: u64) -> &mut Self {
        self.ops.push(Op::Sleep(Duration::from_millis(ms)));
        self
    }

    pub fn send(&mut self, s: Signal) -> &mut Self {
        self.ops.push(Op::Signal(s));
        self
    }

    pub fn panic(&mut self) -> &mut Self {
        self.ops.push(Op::Panic);
        self
    }

    pub fn segfault(&mut self) -> &mut Self {
        self.ops.push(Op::SegFault);
        self
    }
}

impl Script {
    pub fn exec(self) -> i32 {
        let mut rc: i32 = 0;
        let ppid = Pid::parent();

        let Self {
            mut setups,
            mut ops,
        } = self;
        for s in setups.drain(..) {
            s.exec(&mut rc);
        }

        for o in ops.drain(..) {
            o.exec(ppid);
        }

        rc
    }
}

impl Display for Script {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for s in &self.setups {
            if first {
                first = false;
                write!(f, "{}", s)?;
            } else {
                write!(f, " {}", s)?;
            }
        }

        for o in &self.ops {
            if first {
                first = false;
                write!(f, "{}", o)?;
            } else {
                write!(f, " {}", o)?;
            }
        }

        Ok(())
    }
}

impl FromStr for Script {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut setups = Vec::new();
        let mut ops = Vec::new();
        for elem in s.split_whitespace() {
            if let Ok(setup) = elem.parse::<Setup>() {
                setups.push(setup);
            } else if let Ok(op) = elem.parse::<Op>() {
                ops.push(op);
            } else {
                eprintln!("unknown action {elem}");
                return Err(());
            }
        }

        Ok(Self { setups, ops })
    }
}
