//! An auto-generated set of NFS wire types.
//!
//! Do NOT modify the generated file directly.

#![allow(non_camel_case_types, dead_code)]

use bytes::{Buf, BufMut, Bytes};
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

    #[error("{0}")]
    Unknown(String),
}

pub trait DeserialiserExt {
    type Sliced;
    type TryFrom;

    fn try_u32(&mut self) -> Result<u32, Error>;
    fn try_u64(&mut self) -> Result<u64, Error>;
    fn try_i32(&mut self) -> Result<i32, Error>;
    fn try_i64(&mut self) -> Result<i64, Error>;
    fn try_f32(&mut self) -> Result<f32, Error>;
    fn try_f64(&mut self) -> Result<f64, Error>;
    fn try_bool(&mut self) -> Result<bool, Error>;
    fn try_string(&mut self) -> Result<String, Error>;
    fn try_bytes(&mut self, max: Option<usize>) -> Result<Self::Sliced, Error>;
    fn try_variable_array<T>(&mut self, max: Option<usize>) -> Result<Vec<T>, Error>
    where
        T: TryFrom<Self::TryFrom, Error = Error>;
}

impl DeserialiserExt for Bytes {
    type Sliced = Self;
    type TryFrom = Bytes;

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

    fn try_variable_array<T>(&mut self, max: Option<usize>) -> Result<Vec<T>, Error>
    where
        T: TryFrom<Self, Error = Error>,
    {
        use std::mem::size_of;

        let n = self.try_u32()? as usize;

        if let Some(limit) = max {
            if n > limit {
                return Err(Error::InvalidLength);
            }
        }

        // Calculate how many bytes are required to be in the buffer for n
        // number of T's.
        let byte_len = n * size_of::<T>();

        // Validate the buffer contains enough data
        if self.remaining() < byte_len {
            return Err(Error::InvalidLength);
        }

        // Try and decode n instances of T.
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            let t = T::try_from(self.slice(..size_of::<T>()))?;
            out.push(t);
            self.advance(size_of::<T>());
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[derive(Debug, PartialEq)]
    struct TestStruct {
        a: u32,
    }

    impl TryFrom<Bytes> for TestStruct {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self { a: v.try_u32()? })
        }
    }

    #[test]
    fn test_variable_array_no_max() {
        let mut buf = BytesMut::new();
        buf.put_u32(2); // Len=2
        buf.put_u32(42); // Struct 1
        buf.put_u32(24); // Struct 2
        buf.put_u32(123); // Remaining buffer
        let mut buf = buf.freeze();

        let got = buf.try_variable_array::<TestStruct>(None).unwrap();

        assert_eq!(got.len(), 2);
        assert_eq!(got[0], TestStruct { a: 42 });
        assert_eq!(got[1], TestStruct { a: 24 });
        assert_eq!(buf.as_ref(), &[0, 0, 0, 123]);
    }
}
