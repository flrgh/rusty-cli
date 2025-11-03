// this code is used in the build script and must not have any external dependencies

pub(crate) const RESTY_COMPAT_VAR: &str = "RESTY_CLI_COMPAT_VERSION";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Version {
    pub(crate) maj: u16,
    pub(crate) min: u16,
}

impl Version {
    pub(crate) const fn new(maj: u16, min: u16) -> Self {
        Self { maj, min }
    }
}

impl From<(u16, u16)> for Version {
    fn from(value: (u16, u16)) -> Self {
        Self::new(value.0, value.1)
    }
}

impl From<Version> for (u16, u16) {
    fn from(val: Version) -> Self {
        (val.maj, val.min)
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{0}.{1}", self.maj, self.min)
    }
}

impl Version {
    // this is pretty ridiculous, but hey it's const!
    pub(crate) const fn from_bytes(value: &[u8]) -> Option<Self> {
        /// the most base-10 digits we can fit into a 16 bit integer (65,535)
        const MAX_LEN: usize = 5;

        const fn extract(src: &[u8], len: usize) -> Option<u16> {
            if len == 0 || len > MAX_LEN {
                return None;
            }

            let mut end = len - 1;
            let mut factor = 1u16;
            let mut dst = 0u16;
            loop {
                let digit = (src[end] - b'0') as u16;

                let value = match digit.checked_mul(factor) {
                    Some(value) => value,
                    None => return None,
                };

                match dst.checked_add(value) {
                    Some(new) => dst = new,
                    _ => return None,
                }

                if end == 0 {
                    return Some(dst);
                }

                end -= 1;
                factor = match factor.checked_mul(10) {
                    Some(factor) => factor,
                    None => return None,
                };
            }
        }

        const fn find_digits(src: &[u8]) -> Option<(usize, usize)> {
            let mut index = 0;
            let mut len = 0;

            while index < src.len() && index <= MAX_LEN {
                if index > MAX_LEN {
                    return None;
                }

                let b = src[index];

                match b {
                    b'0'..=b'9' => len += 1,

                    b'.' => return Some((len, index + 1)),

                    _ => return None,
                }

                index += 1;
            }

            Some((len, index))
        }

        // allow a single leading `v`
        let value = if let Some(b'v') = value.first() {
            let (_, rest) = value.split_at(1);
            rest
        } else {
            value
        };

        if value.is_empty() {
            return None;
        }

        match find_digits(value) {
            Some((len, next)) => {
                let maj = if len == 0 {
                    0
                } else {
                    match extract(value, len) {
                        Some(maj) => maj,
                        None => return None,
                    }
                };

                let (_, value) = value.split_at(next);

                if value.is_empty() {
                    Some(Version::new(maj, 0))
                } else {
                    match find_digits(value) {
                        Some((len, _)) => match extract(value, len) {
                            Some(min) => Some(Version::new(maj, min)),
                            None => None,
                        },
                        None => None,
                    }
                }
            }
            None => None,
        }
    }

    pub(crate) const fn from_str(value: &str) -> Option<Self> {
        Self::from_bytes(value.as_bytes())
    }

    pub(crate) fn from_env() -> Option<Result<Self, String>> {
        let Ok(var) = std::env::var(RESTY_COMPAT_VAR) else {
            return None;
        };

        let value = var.trim();
        if value.is_empty() {
            return None;
        }

        Some(Self::from_str(value).ok_or(var))
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.maj.cmp(&other.maj) {
            std::cmp::Ordering::Equal => self.min.cmp(&other.min),
            cmp => cmp,
        }
    }
}

/// The minimum supported resty-cli version
#[allow(dead_code)]
pub(crate) const RESTY_COMPAT_MIN: Version = Version::new(0, 28);

/// The maximum supported resty-cli version
#[allow(dead_code)]
pub(crate) const RESTY_COMPAT_MAX: Version = Version::new(0, 30);
