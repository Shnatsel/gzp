//! Parallel compression.
//!
//! This module provides a implementations of [`Write`] that are backed by an async threadpool that
//! compresses blocks and writes to the underlying writer. This is very similar to how
//! [`pigz`](https://zlib.net/pigz/) works.
//!
//! The supported encodings are:
//!
//! - Gzip: [`pgz::pargz`]
//! - Snap: [`pgz::parsnap`]
//!
//! # References
//!
//! - [ParallelGzip](https://github.com/shevek/parallelgzip/blob/master/src/main/java/org/anarres/parallelgzip/ParallelGZIPOutputStream.java)
//! - [pigz](https://zlib.net/pigz/)
//!
//! # Known Differences from Pigz
//!
//! - Each block has an independent CRC value
//! - There is no continual dictionary for compression, compression is per-block only. On some data
//!   types this could lead to no compression for a given block if the block is small enough or the
//!   data is random enough.
//!
//! # Examples
//!
//! ```
//! # #[cfg(feature = "pargz")] {
//! use std::{env, fs::File, io::Write};
//!
//! use gzp::pargz::ParGz;
//!
//! let mut writer = vec![];
//! let mut par_gz = ParGz::builder(writer).build();
//! par_gz.write_all(b"This is a first test line\n").unwrap();
//! par_gz.write_all(b"This is a second test line\n").unwrap();
//! par_gz.finish().unwrap();
//! # }
//! ```
use std::io::{self};

use bytes::BytesMut;
use flume::{unbounded, Receiver, Sender};
use thiserror::Error;

#[cfg(feature = "pargz")]
pub mod pargz;
#[cfg(feature = "parsnap")]
pub mod parsnap;

/// 128 KB default buffer size, same as pigz
pub(crate) const BUFSIZE: usize = 64 * (1 << 10) * 2;

#[derive(Error, Debug)]
pub enum GzpError {
    #[error("Failed to send over channel.")]
    ChannelSend,
    #[error(transparent)]
    ChannelReceive(#[from] flume::RecvError),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("Unknown")]
    Unknown,
}

/// A message sent from the Par writer to the compressor.
///
/// This message holds both the bytes to be compressed and written, as well as the oneshot channel
/// to send the compressed bytes to the writer.
#[derive(Debug)]
pub(crate) struct Message {
    buffer: BytesMut,
    oneshot: Sender<Result<Vec<u8>, GzpError>>,
}

impl Message {
    /// Create a [`Message`] along with its oneshot channel.
    pub(crate) fn new_parts(buffer: BytesMut) -> (Self, Receiver<Result<Vec<u8>, GzpError>>) {
        let (tx, rx) = unbounded();
        (
            Message {
                buffer,
                oneshot: tx,
            },
            rx,
        )
    }
}
