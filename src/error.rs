//! Error types.

// pub use core::str::Utf8Error;

use crate::{Length, Tag};
use core::{convert::Infallible, fmt};

/// Result type.
pub type Result<T> = core::result::Result<T, Error>;

/// Error type
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Error {
    /// Kind of error
    kind: ErrorKind,

    /// Position inside of message where error occurred
    position: Option<Length>,
}

impl Error {
    /// Create a new [`Error`]
    pub fn new(kind: ErrorKind, position: Length) -> Error {
        Error {
            kind,
            position: Some(position),
        }
    }

    /// Get the [`ErrorKind`] which occurred.
    pub fn kind(self) -> ErrorKind {
        self.kind
    }

    /// Get the position inside of the message where the error occurred.
    pub fn position(self) -> Option<Length> {
        self.position
    }

    /// For errors occurring inside of a nested message, extend the position
    /// count by the location where the nested message occurs.
    pub fn nested(self, nested_position: Length) -> Self {
        // TODO(tarcieri): better handle length overflows occurring in this calculation?
        let position = (nested_position + self.position.unwrap_or_default()).ok();

        Self {
            kind: self.kind,
            position,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)?;

        if let Some(pos) = self.position {
            write!(f, " at SIMPLE-TLV byte {}", pos)?;
        }

        Ok(())
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error {
            kind,
            position: None,
        }
    }
}

impl From<core::convert::Infallible> for Error {
    fn from(_: Infallible) -> Error {
        unreachable!()
    }
}

// impl From<Utf8Error> for Error {
//     fn from(err: Utf8Error) -> Error {
//         Error {
//             kind: ErrorKind::Utf8(err),
//             position: None,
//         }
//     }
// }

// #[cfg(feature = "oid")]
// impl From<const_oid::Error> for Error {
//     fn from(_: const_oid::Error) -> Error {
//         ErrorKind::Oid.into()
//     }
// }

#[cfg(feature = "std")]
impl std::error::Error for ErrorKind {}

/// Error type.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// Operation failed due to previous error
    Failed,

    /// Invalid tag
    InvalidTag {
        /// Raw byte value of the tag
        byte: u8,
    },

    /// Length greater than u16::MAX
    InvalidLength,

    /// Incorrect length for a given field
    Length {
        /// Tag type of the value being decoded
        tag: Tag,
    },

    // /// Message is not canonically encoded
    // Noncanonical,

    // /// Malformed OID
    // Oid,

    /// Integer overflow occurred (library bug!)
    Overflow,

    /// Message is longer than SIMPLE-TLV's limits support
    Overlength,

    /// Undecoded trailing data at end of message
    TrailingData {
        /// Length of the decoded data
        decoded: Length,

        /// Total length of the remaining data left in the buffer
        remaining: Length,
    },

    /// Unexpected end-of-message/nested field when decoding
    Truncated,

    /// Encoded message is shorter than the expected length
    /// (i.e. an `Encodable` impl on a particular type has a buggy `encoded_length`)
    Underlength {
        /// Expected length
        expected: Length,

        /// Actual length
        actual: Length,
    },

    /// Unexpected tag
    UnexpectedTag {
        /// Tag the decoder was expecting (if there is a single such tag).
        ///
        /// `None` if multiple tags are expected/allowed, but the `actual` tag
        /// does not match any of them.
        expected: Option<Tag>,

        /// Actual tag encountered in the message
        actual: Tag,
    },

    // /// Unknown/unsupported tag
    // UnknownTag {
    //     /// Raw byte value of the tag
    //     byte: u8,
    // },

    // /// UTF-8 errors
    // Utf8(Utf8Error),

    // /// Unexpected value
    // Value {
    //     /// Tag of the unexpected value
    //     tag: Tag,
    // },
}

impl ErrorKind {
    /// Annotate an [`ErrorKind`] with context about where it occurred,
    /// returning an error.
    pub fn at(self, position: Length) -> Error {
        Error::new(self, position)
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::Failed => write!(f, "operation failed"),
            ErrorKind::InvalidLength => write!(f, "length greater than protocol maximum"),
            ErrorKind::Length { tag } => write!(f, "incorrect length for {}", tag),
            // ErrorKind::Noncanonical => write!(f, "DER is not canonically encoded"),
            // ErrorKind::Oid => write!(f, "malformed OID"),
            ErrorKind::Overflow => write!(f, "integer overflow"),
            ErrorKind::Overlength => write!(f, "SIMPLE-TLV message is too long"),
            ErrorKind::TrailingData { decoded, remaining } => {
                write!(
                    f,
                    "trailing data at end of SIMPLE-TLV message: decoded {} bytes, {} bytes remaining",
                    decoded, remaining
                )
            }
            ErrorKind::Truncated => write!(f, "SIMPLE-TLV message is truncated"),
            ErrorKind::Underlength { expected, actual } => write!(
                f,
                "SIMPLE-TLV message too short: expected {}, got {}",
                expected, actual
            ),
            ErrorKind::UnexpectedTag { expected, actual } => {
                write!(f, "unexpected SIMPLE-TLV tag: ")?;

                if let Some(tag) = expected {
                    write!(f, "expected {}, ", tag)?;
                }

                write!(f, "got {}", actual)
            }
            ErrorKind::InvalidTag { byte } => {
                write!(f, "invalid SIMPLE-TLV tag: 0x{:02x}", byte)
            }
            // ErrorKind::UnknownTag { byte } => {
            //     write!(f, "unknown/unsupported ASN.1 DER tag: 0x{:02x}", byte)
            // }
            // ErrorKind::Utf8(e) => write!(f, "{}", e),
            // ErrorKind::Value { tag } => write!(f, "malformed ASN.1 DER value for {}", tag),
        }
    }
}
