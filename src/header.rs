//! An auto-generated set of NFS wire types.
//!
//! Do NOT modify the generated file directly.

#![allow(non_camel_case_types, dead_code)]

use bytes::{Buf, BufMut, Bytes};
use std::convert::TryFrom;
use std::fmt::Debug;
use std::mem::size_of;
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
    fn try_string(&mut self, max: Option<usize>) -> Result<String, Error>;
    fn try_variable_bytes(&mut self, max: Option<usize>) -> Result<Self::Sliced, Error>;
    fn try_bytes(&mut self, n: usize) -> Result<Self::Sliced, Error>;
    fn try_variable_array<T>(&mut self, max: Option<usize>) -> Result<Vec<T>, Error>
    where
        T: TryFrom<Self::TryFrom, Error = Error>;
}

impl DeserialiserExt for Bytes {
    type Sliced = Self;
    type TryFrom = Bytes;

    // Try and read a u32 if self contains enough data.
    fn try_u32(&mut self) -> Result<u32, Error> {
        if self.remaining() < size_of::<u32>() {
            return Err(Error::InvalidLength);
        }
        Ok(self.get_u32())
    }

    fn try_u64(&mut self) -> Result<u64, Error> {
        if self.remaining() < size_of::<u64>() {
            return Err(Error::InvalidLength);
        }
        Ok(self.get_u64())
    }

    fn try_i32(&mut self) -> Result<i32, Error> {
        if self.remaining() < size_of::<i32>() {
            return Err(Error::InvalidLength);
        }
        Ok(self.get_i32())
    }

    fn try_i64(&mut self) -> Result<i64, Error> {
        if self.remaining() < size_of::<i64>() {
            return Err(Error::InvalidLength);
        }
        Ok(self.get_i64())
    }

    fn try_f32(&mut self) -> Result<f32, Error> {
        if self.remaining() < size_of::<f32>() {
            return Err(Error::InvalidLength);
        }
        Ok(self.get_f32())
    }

    fn try_f64(&mut self) -> Result<f64, Error> {
        if self.remaining() < size_of::<f64>() {
            return Err(Error::InvalidLength);
        }
        Ok(self.get_f64())
    }

    fn try_bool(&mut self) -> Result<bool, Error> {
        if self.remaining() < size_of::<i32>() {
            return Err(Error::InvalidLength);
        }
        match self.get_i32() {
            0 => Ok(false),
            1 => Ok(false),
            _ => Err(Error::InvalidBoolean),
        }
    }

    fn try_string(&mut self, max: Option<usize>) -> Result<String, Error> {
        let b = self
            .try_variable_bytes(max)?
            .iter()
            .copied()
            .collect::<Vec<u8>>();
        String::from_utf8(b).map_err(|e| e.into())
    }

    /// Try to read an opaque XDR array, prefixed by a length u32 and padded
    /// modulo 4.
    fn try_variable_bytes(&mut self, max: Option<usize>) -> Result<Self::Sliced, Error> {
        let n = self.try_u32()? as usize;

        if let Some(limit) = max {
            if n > limit {
                return Err(Error::InvalidLength);
            }
        }

        self.try_bytes(n)
    }

    /// Try to read an opaque XDR array with a fixed length and padded modulo 4.
    fn try_bytes(&mut self, n: usize) -> Result<Self::Sliced, Error> {
        // Validate the buffer contains enough data
        if self.remaining() < n {
            return Err(Error::InvalidLength);
        }

        let data = self.slice(..n);

        // All byte arrays are padded modulo 4.
        let padding = n % 4;

        // Advance the buffer cursor
        self.advance(data.len() + padding);

        Ok(data)
    }

    fn try_variable_array<T>(&mut self, max: Option<usize>) -> Result<Vec<T>, Error>
    where
        T: TryFrom<Self, Error = Error>,
    {
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

        // All byte arrays are padded modulo 4.
        let padding = byte_len % 4;
        self.advance(padding);

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[derive(Debug, PartialEq)]
    #[repr(C)]
    struct TestStruct {
        a: u32,
    }

    impl TryFrom<Bytes> for TestStruct {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self { a: v.try_u32()? })
        }
    }

    #[derive(Debug, PartialEq)]
    #[repr(C)]
    struct UnalignedStruct {
        a: u8,
    }

    impl TryFrom<Bytes> for UnalignedStruct {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let s = v.slice(..1);
            v.advance(1);
            Ok(Self { a: s.as_ref()[0] })
        }
    }

    #[test]
    fn test_variable_array_no_max() {
        let mut buf = BytesMut::new();
        buf.put_u32(4); // Len=4
        buf.put_u8(1); // Struct 1
        buf.put_u8(2); // Struct 2
        buf.put_u8(3); // Struct 3
        buf.put_u8(4); // Struct 4
        buf.put_u32(123); // Remaining buffer
        let mut buf = buf.freeze();

        let got = buf.try_variable_array::<UnalignedStruct>(None).unwrap();

        assert_eq!(got.len(), 4);
        assert_eq!(got[0], UnalignedStruct { a: 1 });
        assert_eq!(got[1], UnalignedStruct { a: 2 });
        assert_eq!(got[2], UnalignedStruct { a: 3 });
        assert_eq!(got[3], UnalignedStruct { a: 4 });

        assert_eq!(buf.len(), 4);
        assert_eq!(buf.as_ref(), &[0, 0, 0, 123]);
    }

    #[test]
    fn test_variable_array_no_max_with_padding() {
        let mut buf = BytesMut::new();
        buf.put_u32(2); // Len=4
        buf.put_u8(1); // Struct 1
        buf.put_u8(2); // Struct 2
        buf.put_u8(0); // Padding
        buf.put_u8(0); // Padding
        buf.put_u32(123); // Remaining buffer
        let mut buf = buf.freeze();

        let got = buf.try_variable_array::<UnalignedStruct>(None).unwrap();

        assert_eq!(got.len(), 2);
        assert_eq!(got[0], UnalignedStruct { a: 1 });
        assert_eq!(got[1], UnalignedStruct { a: 2 });

        assert_eq!(buf.len(), 4);
        assert_eq!(buf.as_ref(), &[0, 0, 0, 123]);
    }

    #[test]
    fn test_try_variable_bytes_no_max() {
        let mut buf = BytesMut::new();
        buf.put_u32(8); // Len=8
        buf.put([1, 2, 3, 4, 5, 6, 7, 8].as_ref());
        let mut buf = buf.freeze();

        let got = buf.try_variable_bytes(None).unwrap();

        assert_eq!(got.len(), 8);
        assert_eq!(got.as_ref(), &[1, 2, 3, 4, 5, 6, 7, 8]);

        assert_eq!(buf.as_ref(), &[]);
        assert_eq!(buf.remaining(), 0);
    }

    #[test]
    fn test_try_variable_bytes_no_max_with_padding() {
        let mut buf = BytesMut::new();
        buf.put_u32(6); // Len=8
        buf.put([1, 2, 3, 4, 5, 6, 0, 0].as_ref());
        let mut buf = buf.freeze();

        let got = buf.try_variable_bytes(None).unwrap();

        assert_eq!(got.len(), 6);
        assert_eq!(got.as_ref(), &[1, 2, 3, 4, 5, 6]);

        assert_eq!(buf.as_ref(), &[]);
        assert_eq!(buf.remaining(), 0);
    }
}
