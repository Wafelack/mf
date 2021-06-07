use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub struct Error(pub String);
impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
use std::io;
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self(e.to_string())
    }
}
pub type Result<T> = std::result::Result<T, Error>;
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        std::result::Result::Err(crate::errors::Error(format_args!($($arg)*).to_string()))
    }
}
