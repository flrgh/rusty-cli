use mktemp::Temp;
use std::fmt::{Debug, Display, Formatter, Result as fmtResult};
use std::fs;
use std::path::PathBuf;

pub(crate) struct Prefix {
    pub(crate) root: PathBuf,
    pub(crate) conf: PathBuf,
    pub(crate) logs: PathBuf,
    _tmp: Temp,
}

impl Debug for Prefix {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmtResult {
        write!(f, "{}", self.root.to_str().unwrap())
    }
}

impl Display for Prefix {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmtResult {
        write!(f, "{}", self.root.to_str().unwrap())
    }
}

impl Prefix {
    pub(crate) fn new() -> Result<Self, std::io::Error> {
        let tmp = Temp::new_dir().unwrap();

        //let root = tmp.to_path_buf();
        let root = PathBuf::from("./test");
        let conf = root.join("conf");
        let logs = root.join("logs");

        fs::create_dir_all(&root)?;
        fs::create_dir_all(&conf)?;
        fs::create_dir_all(&logs)?;

        Ok(Prefix {
            root,
            conf,
            logs,
            _tmp: tmp,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ValueWithIndex {
    pub(crate) value: String,
    pub(crate) index: usize,
}

impl PartialOrd for ValueWithIndex {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.index.partial_cmp(&other.index)
    }
}

impl Ord for ValueWithIndex {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.index.cmp(&other.index)
    }
}

impl From<(usize, &String)> for ValueWithIndex {
    fn from(idx_val: (usize, &String)) -> Self {
        ValueWithIndex {
            index: idx_val.0,
            value: idx_val.1.to_owned(),
        }
    }
}

impl From<ValueWithIndex> for String {
    fn from(val: ValueWithIndex) -> Self {
        val.value
    }
}
