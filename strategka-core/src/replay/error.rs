use crate::replay::MAGIC_BYTES;
use core::fmt::Debug;
use nom::{
    error::{ContextError, ErrorKind, ParseError},
    Needed,
};
use thiserror::Error;

use crate::Turn;

/// Error that are specific for replays
#[derive(Debug, Error)]
pub enum GenericError<I: Debug> {
    #[error("Cannot record non monotonic step count. Last step {0}, tried to add new step {1}")]
    IncoherentTurn(Turn, Turn),
    #[error("The encoder or decoder failed to IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Invalid magic bytes in header: {0:?}, expected {:?}", MAGIC_BYTES)]
    InvalidMagic([u8; 4]),
    #[error("Usupported core version of replay format: {0}")]
    UnsupportedCoreVersion(u32),
    #[error("Unsupported game version of replay format: {0}")]
    UnsupportedGameVersion(u32),
    #[error("There is input with length 0 in replay turn")]
    MissingTurnInput,
    #[error("Parsing error {1:?} for input: {0:?}")]
    Parsing(I, ErrorKind),
    #[error("Length prefixed block has invalid length. Found {0}, the input has only {1} bytes")]
    InvalidLength(usize, usize),
    #[error("Failed to encode cbor: {0}")]
    Encoder(#[from] ciborium::ser::Error<std::io::Error>),
    #[error("Failed to decode cbor: {0}")]
    Decoder(#[from] ciborium::de::Error<std::io::Error>),
    #[error("Context {0}. {1}")]
    Context(&'static str, Box<Self>),
    #[error("Parsing failed as incomplete input provided. Needed {0:?}")]
    Incomplete(Needed),
}

/// Error that shares part of original buffer
pub type Error<'a> = GenericError<&'a [u8]>;

/// Error that copies part of original buffer
pub type ErrorOwned = GenericError<Vec<u8>>;

/// Shortcut for results with replay errors
pub type Result<'a, T> = std::result::Result<T, Error<'a>>;

/// Shortcut for results with replay errors
pub type ResultOwned<T> = std::result::Result<T, ErrorOwned>;

impl<'a> GenericError<&'a [u8]> {
    pub fn into_owned(self) -> GenericError<Vec<u8>> {
        match self {
            GenericError::IncoherentTurn(t1, t2) => GenericError::IncoherentTurn(t1, t2),
            GenericError::IoError(e) => GenericError::IoError(e),
            GenericError::InvalidMagic(v) => GenericError::InvalidMagic(v),
            GenericError::UnsupportedCoreVersion(v) => GenericError::UnsupportedCoreVersion(v),
            GenericError::UnsupportedGameVersion(v) => GenericError::UnsupportedGameVersion(v),
            GenericError::MissingTurnInput => GenericError::MissingTurnInput,
            GenericError::Parsing(v, k) => GenericError::Parsing(v.to_owned(), k),
            GenericError::InvalidLength(l1, l2) => GenericError::InvalidLength(l1, l2),
            GenericError::Encoder(e) => GenericError::Encoder(e),
            GenericError::Decoder(e) => GenericError::Decoder(e),
            GenericError::Context(v, other) => {
                GenericError::Context(v, Box::new(other.into_owned()))
            }
            GenericError::Incomplete(needed) => GenericError::Incomplete(needed),
        }
    }
}

impl<'a> ParseError<&'a [u8]> for Error<'a> {
    fn from_error_kind(input: &'a [u8], kind: ErrorKind) -> Self {
        Error::Parsing(input, kind)
    }

    fn append(_: &[u8], _: ErrorKind, other: Self) -> Self {
        other
    }
}

impl<'a> ContextError<&'a [u8]> for Error<'a> {
    fn add_context(_input: &'a [u8], ctx: &'static str, other: Self) -> Self {
        Error::Context(ctx, Box::new(other))
    }
}
