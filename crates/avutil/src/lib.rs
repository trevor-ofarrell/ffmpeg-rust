//! Shared primitives for the FFmpeg-compatible Rust implementation.

pub mod bitreader;
pub mod bitwriter;
pub mod byteio;
pub mod dict;
pub mod error;
pub mod frame;
pub mod logging;
pub mod packet;
pub mod rational;
pub mod timebase;

pub use bitreader::BitReader;
pub use bitwriter::BitWriter;
pub use byteio::{ByteReader, ByteWriter};
pub use dict::{Dictionary, DictionaryEntry, DictionarySet, MatchMode, SetMode};
pub use error::{AvError, AvErrorKind, AvResult};
pub use frame::{AudioFrame, Frame, FrameData, VideoFrame};
pub use logging::{LogLevel, LogRecord, Logger};
pub use packet::{Packet, PacketFlags, SideData};
pub use rational::Rational;
pub use timebase::{rescale_q, rescale_q_rnd, Rounding};
