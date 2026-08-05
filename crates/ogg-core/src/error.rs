//! Error type for the Ogg reader — Kryx's own boundary.
//!
//! We expose our own error rather than leaking `std::io::Error`, so the public
//! API is stable and independent of how bytes are sourced.

use std::fmt;

/// An error while reading Ogg.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OggError {
    /// The capture pattern ("OggS") was not found where a page was expected.
    BadCapturePattern { offset: usize },
    /// The page's stream-structure version byte was not 0.
    UnsupportedVersion { version: u8, offset: usize },
    /// The data ended in the middle of a page (header, segment table, or body).
    Truncated {
        context: &'static str,
        offset: usize,
    },
    /// The page's stored CRC did not match the computed CRC.
    CrcMismatch {
        offset: usize,
        stored: u32,
        computed: u32,
    },
}

impl fmt::Display for OggError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OggError::BadCapturePattern { offset } => {
                write!(f, "bad Ogg capture pattern at byte {offset}")
            }
            OggError::UnsupportedVersion { version, offset } => {
                write!(f, "unsupported Ogg version {version} at byte {offset}")
            }
            OggError::Truncated { context, offset } => {
                write!(f, "truncated Ogg {context} at byte {offset}")
            }
            OggError::CrcMismatch {
                offset,
                stored,
                computed,
            } => write!(
                f,
                "Ogg CRC mismatch at page starting byte {offset}: \
                 stored {stored:#010x}, computed {computed:#010x}"
            ),
        }
    }
}

impl std::error::Error for OggError {}

/// Convenience alias for results in this crate.
pub type OggResult<T> = Result<T, OggError>;
