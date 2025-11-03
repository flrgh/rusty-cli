use std::fs::File;
use std::io;
use std::path::PathBuf;
use test_utils::nginx::Nginx;

fn main() {
    let nginx = Nginx::try_from_args();
    let mut stdout = io::stdout().lock();
    let mut fh = File::open(nginx.conf_filename()).expect("opening nginx.conf for reading");
    let _ = io::copy(&mut fh, &mut stdout).expect("copying nginx.conf to stdout");
}
