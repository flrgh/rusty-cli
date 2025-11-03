use std::path::PathBuf;

#[derive(Debug)]
pub struct Nginx {
    prefix: PathBuf,
    conf: PathBuf,
}

impl Nginx {
    pub fn new(prefix: PathBuf, conf: Option<PathBuf>) -> Self {
        let conf = conf.unwrap_or(PathBuf::from("conf/nginx.conf"));
        Self { prefix, conf }
    }

    pub fn try_from_args() -> Self {
        let mut args = std::env::args();

        let mut prefix = None;
        let mut conf = None;

        while let Some(opt) = args.next() {
            match opt.as_str() {
                "-p" => {
                    prefix = Some(args.next().expect("`-p` opt with no arg"));
                }

                "-c" => {
                    conf = Some(PathBuf::from(args.next().expect("`-c` opt with no arg")));
                }

                _ => {}
            }
        }

        let prefix = prefix.expect("no prefix directory provided");
        Self::new(prefix.into(), conf)
    }

    pub fn conf_filename(&self) -> PathBuf {
        if self.conf.is_absolute() {
            self.conf.clone()
        } else {
            self.prefix.join(&self.conf)
        }
    }
}
