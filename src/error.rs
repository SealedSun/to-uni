
use std::io;
use std::fmt::{self,Display, Debug};
use std::error::{Error};

use ::yaml;

/// Error type for the to-uni program.
pub struct UniError {
    /// Coarse-grained error code based on the technical kind of error. 
    /// Range 0-25
    ///   00: user error
    code_major: u8,

    /// Detailed error code to distinguish the situation in which particular technical errors occur.
    /// Range: 0-9
    code_minor: u8,

    /// The actual error data.
    data: UniErrorData
}

impl UniError {
    pub fn error_code(&self) -> u8 {
        self.code_major*10 + self.code_minor
    }

    pub fn new(minor: u8, data: UniErrorData) -> UniError {
        let (major,_) = data.default_code_major_minor();
        UniError {
            code_major: major, code_minor: minor, data
        }
    }

    pub fn with_minor(mut self, minor: u8) -> Self {
        self.code_minor = minor;
        self
    }
}

pub mod code {
    pub mod fsio {
        pub static INPUT: u8 = 2;
        pub static OUTPUT: u8 = 3;
        pub static OUTPUT_BACKUP: u8 = 4;
        pub static CONFIG: u8 = 5;
    }
    pub mod internal {
        pub static MISC: u8 = 8;
    }
    pub mod usage {
        pub static MISSING_OUTPUT_FILE_NAME: u8 = 4;
        pub static MISSING_OUTPUT: u8 =  5;
        pub static INPUT_NOT_A_FILE: u8 = 6;
        pub static NO_CONFIG_FILE: u8 = 7;
        pub static INVALID_CONFIG_FILE: u8 = 8;
    }
}

pub fn usage(message: String) -> UniError {
    let data = UniErrorData::Usage(message);
    let (minor,major) = data.default_code_major_minor();
    UniError {
        code_minor: minor, code_major: major, data
    }
}

/// Error data specific to particular error kinds.
#[derive(Debug)]
pub enum UniErrorData {
    /// IO error related to a file system path.
    FsIo(String, io::Error),
    /// General IO error
    Io(io::Error),
    Internal(String),
    Usage(String),
    /// YAML file path
    YamlScan(String, yaml::ScanError)
}

impl UniErrorData {
    pub fn default_code_major_minor(&self) -> (u8,u8) {
        match *self {
            UniErrorData::Io(_) => (1,0),
            UniErrorData::FsIo(_,_) => (2,0),
            UniErrorData::Internal(_) => (9,0),
            UniErrorData::Usage(_) => (0,1),
            UniErrorData::YamlScan(_,_) => (3,0)
        }
    }
}

impl Error for UniError {
    fn description(&self) -> &str {
        match self.data {
            UniErrorData::Io(_) => "General IO error.",
            UniErrorData::FsIo(_,_) => "File system IO error.",
            UniErrorData::Internal(_) => "Internal error.",
            UniErrorData::Usage(_) => "Usage error.",
            UniErrorData::YamlScan(_,_) => "YAML parsing error."
        }
    }
    fn cause(&self) -> Option<&Error> {
        match self.data {
            UniErrorData::Io(ref e) => Some(e),
            UniErrorData::FsIo(_, ref e) => Some(e),
            UniErrorData::Internal(_) => None,
            UniErrorData::Usage(_) => None,
            UniErrorData::YamlScan(_, ref e) => Some(e)
        }
    }
}

impl Display for UniError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{} ", self.description())?;
        match self.data {
            UniErrorData::Io(ref e) => write!(f, "{}", e),
            UniErrorData::FsIo(ref path, ref e) => write!(f, "{} Path: {}", e, path),
            UniErrorData::Internal(ref m) => write!(f, "{}", m),
            UniErrorData::Usage(ref m) => write!(f, "{}", m),
            UniErrorData::YamlScan(ref path, ref e) => write!(f, "{} Path: {}", e, path)
        }
    }
}

impl Debug for UniError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{} Code: {} Data: {:?}", self.description(), self.error_code(), self.data)
    }
}

//////////////////////////////////////////////////////////////////////////////////////////////////

pub trait DetailedFrom<E, D> {
    fn detailed_from(err: E, details: D) -> Self;
}

#[macro_export]
macro_rules! try_ {
    ($expr:expr, $($details:expr),+) => (match $expr {
        ::std::result::Result::Ok(val) => val,
        ::std::result::Result::Err(err) => {
            return ::std::result::Result::Err(
                $crate::error::DetailedFrom::detailed_from(err, ($($details),+) ))
        }
    })
}

macro_rules! from_result_ {
    ($expr:expr, $($details:expr),+) => (match $expr {
        ::std::result::Result::Err(err) => {
            ::std::result::Result::Err(
                $crate::error::DetailedFrom::detailed_from(err, ($($details),+) ))
            },
        ::std::result::Result::Ok(v) => Ok(v)
    })
}

macro_rules! from_ {
    ($expr:expr, $($details:expr),+) => (
        $crate::error::DetailedFrom::detailed_from($expr, ($($details),*) )
    )
}

impl From<io::Error> for UniError {
    fn from(err: io::Error) -> UniError {
        let data = UniErrorData::Io(err);
        let (major,minor) = data.default_code_major_minor();
        UniError {
            code_major: major, code_minor: minor, data: data
        }
    }
}

impl DetailedFrom<io::Error, (String, u8)> for UniError {
    fn detailed_from(err: io::Error, details: (String, u8)) -> UniError {
        let data = UniErrorData::FsIo(details.0, err);
        let (major,_) = data.default_code_major_minor();
        UniError {
            code_major: major, code_minor: details.1, data
        }
    }
}

impl DetailedFrom<String, u8> for UniError {
    fn detailed_from(s: String, minor: u8) -> UniError {
        let data = UniErrorData::Internal(s);
        let (major,_) = data.default_code_major_minor();
        UniError {
            code_major: major, code_minor: minor, data
        }
    }
}

impl <'a> DetailedFrom<&'a str, u8> for UniError {
    fn detailed_from(s: &'a str, minor: u8) -> UniError {
        let data = UniErrorData::Internal(s.to_string());
        let (major,_) = data.default_code_major_minor();
        UniError {
            code_major: major, code_minor: minor, data
        }
    }
}

impl DetailedFrom<yaml::ScanError, String> for UniError {
    fn detailed_from(err: yaml::ScanError, path: String) -> UniError {
        let data = UniErrorData::YamlScan(path, err);
        let (major,minor) = data.default_code_major_minor();
        UniError {
            code_major: major, code_minor: minor, data
        }
    }
}
