use std::{
    fmt::{self, Display, Formatter},
    io,
};

#[derive(Debug)]
pub enum Error {
    InvalidSof(u8),
    InvalidFcs(u8),
    InvalidSubsystem(u8),
    InvalidCmdType(u8),
    Io(io::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSof(sof) => write!(f, "Invalid SOF: {}", sof),
            Self::InvalidFcs(check) => write!(f, "Invalid check: {}", check),
            Self::InvalidSubsystem(subsys) => write!(f, "Invalid subsystem: {}", subsys),
            Self::InvalidCmdType(cmd_type) => write!(f, "Invalid command type: {}", cmd_type),
            Self::Io(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}
