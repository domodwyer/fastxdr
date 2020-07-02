//! An auto-generated set of NFS wire types.
//!
//! Do NOT modify the generated file directly.

#![allow(non_camel_case_types, dead_code)]

use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::convert::TryFrom;
use std::fmt::Debug;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum Error {
    #[error("invalid message length")]
    InvalidLength,

    #[error("non-utf8 characters in string: {0}")]
    NonUtf8String(#[from] std::string::FromUtf8Error),

    #[error("invalid boolean value")]
    InvalidBoolean,

    #[error("unknown enum variant {0}")]
    UnknownVariant(i32),

    #[error("unknown option variant {0}")]
    UnknownOptionVariant(u32),
}

pub trait DeserialiserExt {
    type Sliced;

    fn try_u32(&mut self) -> Result<u32, Error>;
    fn try_u64(&mut self) -> Result<u64, Error>;
    fn try_i32(&mut self) -> Result<i32, Error>;
    fn try_i64(&mut self) -> Result<i64, Error>;
    fn try_f32(&mut self) -> Result<f32, Error>;
    fn try_f64(&mut self) -> Result<f64, Error>;
    fn try_bool(&mut self) -> Result<bool, Error>;
    fn try_string(&mut self) -> Result<String, Error>;
    fn try_bytes(&mut self, max: Option<usize>) -> Result<Self::Sliced, Error>;
}

impl DeserialiserExt for Bytes {
    type Sliced = Self;

    // Try and read a u32 if self contains enough data.
    fn try_u32(&mut self) -> Result<u32, Error> {
        if self.remaining() < std::mem::size_of::<u32>() {
            return Err(Error::InvalidLength);
        }
        Ok(self.get_u32())
    }

    fn try_u64(&mut self) -> Result<u64, Error> {
        if self.remaining() < std::mem::size_of::<u64>() {
            return Err(Error::InvalidLength);
        }
        Ok(self.get_u64())
    }

    fn try_i32(&mut self) -> Result<i32, Error> {
        if self.remaining() < std::mem::size_of::<i32>() {
            return Err(Error::InvalidLength);
        }
        Ok(self.get_i32())
    }

    fn try_i64(&mut self) -> Result<i64, Error> {
        if self.remaining() < std::mem::size_of::<i64>() {
            return Err(Error::InvalidLength);
        }
        Ok(self.get_i64())
    }

    fn try_f32(&mut self) -> Result<f32, Error> {
        if self.remaining() < std::mem::size_of::<f32>() {
            return Err(Error::InvalidLength);
        }
        Ok(self.get_f32())
    }

    fn try_f64(&mut self) -> Result<f64, Error> {
        if self.remaining() < std::mem::size_of::<f64>() {
            return Err(Error::InvalidLength);
        }
        Ok(self.get_f64())
    }

    fn try_bool(&mut self) -> Result<bool, Error> {
        if self.remaining() < std::mem::size_of::<i32>() {
            return Err(Error::InvalidLength);
        }
        match self.get_i32() {
            0 => Ok(false),
            1 => Ok(false),
            _ => Err(Error::InvalidBoolean),
        }
    }

    fn try_string(&mut self) -> Result<String, Error> {
        let b = self.try_bytes(None)?.iter().copied().collect::<Vec<u8>>();
        String::from_utf8(b).map_err(|e| e.into())
    }

    /// Try to read an opaque XDR array, prefixed by a length u32.
    fn try_bytes(&mut self, max: Option<usize>) -> Result<Self::Sliced, Error> {
        let len = self.try_u32()? as usize;

        if let Some(limit) = max {
            if len > limit {
                return Err(Error::InvalidLength);
            }
        }

        if self.remaining() < len {
            return Err(Error::InvalidLength);
        }

        let data = self.slice(..len);
        self.advance(data.len());

        Ok(data)
    }
}
