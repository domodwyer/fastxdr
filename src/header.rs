pub mod xdr {
    //! An auto-generated set of NFS wire types.
    //!
    //! Do NOT modify the generated file directly.

    #![allow(non_camel_case_types, dead_code, unused_mut, unreachable_patterns)]

    use std::convert::TryFrom;
    use std::fmt::Debug;
    use std::mem::size_of;
    
    use fastxdr::bytes::{Buf, Bytes};
    use fastxdr::thiserror::Error;

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
        type Sliced: WireSize + IntoIterator<Item = u8>;
        type TryFrom;

        fn read_u32(&mut self) -> Result<u32, Error>;
        fn read_u64(&mut self) -> Result<u64, Error>;
        fn read_i32(&mut self) -> Result<i32, Error>;
        fn read_i64(&mut self) -> Result<i64, Error>;
        fn read_f32(&mut self) -> Result<f32, Error>;
        fn read_f64(&mut self) -> Result<f64, Error>;
        fn read_bool(&mut self) -> Result<bool, Error>;
        fn read_bytes(&mut self, n: usize) -> Result<Self::Sliced, Error>;
        fn read_variable_array<T>(&mut self, max: Option<usize>) -> Result<Vec<T>, Error>
        where
            T: TryFrom<Self::TryFrom, Error = Error> + WireSize;

        /// Try to read an opaque XDR array, prefixed by a length u32 and padded
        /// modulo 4.
        fn read_variable_bytes(&mut self, max: Option<usize>) -> Result<Self::Sliced, Error> {
            let n = self.read_u32()? as usize;

            if let Some(limit) = max {
                if n > limit {
                    return Err(Error::InvalidLength);
                }
            }

            self.read_bytes(n)
        }

        /// Reads a variable length UTF8-compatible string from the buffer.
        fn read_string(&mut self, max: Option<usize>) -> Result<String, Error> {
            let b = self
                .read_variable_bytes(max)?
                .into_iter()
                .collect::<Vec<u8>>();
            String::from_utf8(b).map_err(|e| e.into())
        }
    }

    impl DeserialiserExt for Bytes {
        type Sliced = Self;
        type TryFrom = Bytes;

        // Try and read a u32 if self contains enough data.
        fn read_u32(&mut self) -> Result<u32, Error> {
            if self.remaining() < size_of::<u32>() {
                return Err(Error::InvalidLength);
            }
            Ok(self.get_u32())
        }

        fn read_u64(&mut self) -> Result<u64, Error> {
            if self.remaining() < size_of::<u64>() {
                return Err(Error::InvalidLength);
            }
            Ok(self.get_u64())
        }

        fn read_i32(&mut self) -> Result<i32, Error> {
            if self.remaining() < size_of::<i32>() {
                return Err(Error::InvalidLength);
            }
            Ok(self.get_i32())
        }

        fn read_i64(&mut self) -> Result<i64, Error> {
            if self.remaining() < size_of::<i64>() {
                return Err(Error::InvalidLength);
            }
            Ok(self.get_i64())
        }

        fn read_f32(&mut self) -> Result<f32, Error> {
            if self.remaining() < size_of::<f32>() {
                return Err(Error::InvalidLength);
            }
            Ok(self.get_f32())
        }

        fn read_f64(&mut self) -> Result<f64, Error> {
            if self.remaining() < size_of::<f64>() {
                return Err(Error::InvalidLength);
            }
            Ok(self.get_f64())
        }

        fn read_bool(&mut self) -> Result<bool, Error> {
            if self.remaining() < size_of::<i32>() {
                return Err(Error::InvalidLength);
            }
            match self.get_i32() {
                0 => Ok(false),
                1 => Ok(true),
                _ => Err(Error::InvalidBoolean),
            }
        }

        /// Try to read an opaque XDR array with a fixed length and padded modulo 4.
        fn read_bytes(&mut self, n: usize) -> Result<Self::Sliced, Error> {
            // Validate the buffer contains enough data
            if self.remaining() < n {
                return Err(Error::InvalidLength);
            }

            let data = self.slice(..n);

            // Advance the buffer cursor, including any padding.
            self.advance(n + pad_length(n));

            Ok(data)
        }

        fn read_variable_array<T>(&mut self, max: Option<usize>) -> Result<Vec<T>, Error>
        where
            T: TryFrom<Self, Error = Error> + WireSize,
        {
            let n = self.read_u32()? as usize;

            if let Some(limit) = max {
                if n > limit {
                    return Err(Error::InvalidLength);
                }
            }

            // Try and decode n instances of T.
            let mut sum = 0;
            let mut out = Vec::with_capacity(n);
            for _ in 0..n {
                let t = T::try_from(self.clone())?;
                if self.remaining() < t.wire_size() {
                    return Err(Error::InvalidLength);
                }
                self.advance(t.wire_size());
                sum += t.wire_size();
                out.push(t);
            }

            self.advance(pad_length(sum));

            Ok(out)
        }
    }

    pub trait WireSize {
        fn wire_size(&self) -> usize;
    }

    macro_rules! wiresize_fixed {
        ($size:literal, $($type:ty),+) => {
            $(
                impl WireSize for $type {
                    fn wire_size(&self) -> usize {
                        $size
                    }
                }
            )+
        };
    }

    wiresize_fixed!(1, u8);
    wiresize_fixed!(4, u32, i32, f32, bool);
    wiresize_fixed!(8, u64, i64, f64);

    impl WireSize for Bytes {
        fn wire_size(&self) -> usize {
            self.len()
        }
    }

    impl<T> WireSize for Vec<T>
    where
        T: WireSize,
    {
        fn wire_size(&self) -> usize {
            // Element count prefix of 4 bytes, plus the individual element lengths
            // (which may vary between elements).
            let x = self.iter().map(|v| v.wire_size()).sum::<usize>();
            4 + x + pad_length(x)
        }
    }

    impl<T> WireSize for [T]
    where
        T: WireSize,
    {
        fn wire_size(&self) -> usize {
            // Individual element lengths (which may vary between elements) without
            // a length byte as [T] is for fixed size arrays.
            let x = self.iter().map(|v| v.wire_size()).sum::<usize>();
            x + pad_length(x)
        }
    }

    impl<T> WireSize for Option<T>
    where
        T: WireSize,
    {
        fn wire_size(&self) -> usize {
            4 + match self {
                Some(inner) => inner.wire_size(),
                None => 0,
            }
        }
    }

    impl<T> WireSize for Box<T>
    where
        T: WireSize,
    {
        fn wire_size(&self) -> usize {
            use std::ops::Deref;
            self.deref().wire_size()
        }
    }

    impl WireSize for String {
        fn wire_size(&self) -> usize {
            4 + self.len() + pad_length(self.len())
        }
    }

    /// Return the amount of padding needed for a value of l bytes in length.
    #[inline]
    fn pad_length(l: usize) -> usize {
        if l % 4 == 0 {
            return 0;
        }
        4 - (l % 4)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use fastxdr::bytes::{BufMut, BytesMut};

        #[derive(Debug, PartialEq)]
        struct TestStruct {
            a: u32,
        }

        impl TryFrom<Bytes> for TestStruct {
            type Error = Error;

            fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
                Ok(Self { a: v.read_u32()? })
            }
        }

        impl WireSize for TestStruct {
            fn wire_size(&self) -> usize {
                self.a.wire_size()
            }
        }

        #[derive(Debug, PartialEq)]
        struct VariableSizedStruct {
            a: Vec<u32>,
        }

        impl TryFrom<Bytes> for VariableSizedStruct {
            type Error = Error;

            fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
                // Stub, always has a len of 2
                let x = v.read_u32()?;
                if x != 2 {
                    panic!("expected len of 2, got {}", x);
                }
                Ok(Self {
                    a: vec![v.read_u32()?, v.read_u32()?],
                })
            }
        }

        impl WireSize for VariableSizedStruct {
            fn wire_size(&self) -> usize {
                self.a.wire_size()
            }
        }

        #[derive(Debug, PartialEq)]
        struct UnalignedStruct {
            a: u8,
        }

        wiresize_fixed!(1, UnalignedStruct);

        impl TryFrom<Bytes> for UnalignedStruct {
            type Error = Error;

            fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
                let s = v.slice(..1);
                v.advance(1);
                Ok(Self { a: s.as_ref()[0] })
            }
        }

        #[test]
        fn test_pad_length() {
            assert_eq!(pad_length(0), 0);
            assert_eq!(pad_length(1), 3);
            assert_eq!(pad_length(2), 2);
            assert_eq!(pad_length(3), 1);
            assert_eq!(pad_length(4), 0);
        }

        #[test]
        fn test_wire_size_basic_types() {
            assert_eq!((42 as u8).wire_size(), 1);
            assert_eq!((42 as u32).wire_size(), 4);
            assert_eq!((42 as i32).wire_size(), 4);
            assert_eq!((42 as u64).wire_size(), 8);
            assert_eq!((42 as i64).wire_size(), 8);
            assert_eq!((42_f32).wire_size(), 4);
            assert_eq!((42_f64).wire_size(), 8);

            // Length prefix of 4 bytes, plus data 5 bytes, plus padding to mod 4
            assert_eq!(String::from("test!").wire_size(), 4 + 5 + 3);

            let mut b = Bytes::new();
            assert_eq!(b.wire_size(), 0);

            let b = BytesMut::new().freeze();
            assert_eq!(b.wire_size(), 0);

            let b = BytesMut::from("test!").freeze();
            assert_eq!(b.wire_size(), 5);

            let b: &[u8] = &[1, 2, 3, 4];
            assert_eq!(b.wire_size(), 4);

            let b: &[u8] = &[1, 2, 3, 4, 5];
            assert_eq!(b.wire_size(), 8); // Padded
        }

        #[test]
        fn test_wire_size_vec() {
            let v1: Vec<u32> = vec![1, 2, 3, 4];
            assert_eq!(v1.wire_size(), 4 * 5);

            let v2: Vec<u64> = vec![1, 2, 3, 4];
            assert_eq!(v2.wire_size(), (8 * 4) + 4);
        }

        #[test]
        fn test_wire_size_array() {
            let v1: [u32; 4] = [1, 2, 3, 4];
            assert_eq!(v1.wire_size(), 4 * 4);

            let v2: [u64; 4] = [1, 2, 3, 4];
            assert_eq!(v2.wire_size(), 8 * 4);
        }

        #[test]
        fn test_variable_array_variable_len_struct() {
            let mut buf = BytesMut::new();
            buf.put_u32(2); // 2 structs

            buf.put_u32(2); // This struct has 2 values
            buf.put_u32(1); // Struct 1
            buf.put_u32(2); // Struct 2

            buf.put_u32(2); // This struct has 2 values
            buf.put_u32(3); // Struct 1
            buf.put_u32(4); // Struct 2

            buf.put_u32(123); // Remaining buffer
            let mut buf = buf.freeze();

            let got = buf
                .read_variable_array::<VariableSizedStruct>(None)
                .unwrap();

            assert_eq!(got.len(), 2);
            assert_eq!(
                got.wire_size(),
                4 + // Variable array length prefix

                4 + // First struct array length prefix
                8 + // First struct data

                4 + // Second struct array length prefix
                8 // Second struct data
            );
            assert_eq!(got[0], VariableSizedStruct { a: vec![1, 2] });

            assert_eq!(buf.len(), 4);
            assert_eq!(buf.as_ref(), &[0, 0, 0, 123]);
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

            let got = buf.read_variable_array::<UnalignedStruct>(None).unwrap();

            assert_eq!(got.len(), 4);
            assert_eq!(got.wire_size(), 4 + 4); // Inner vecs + vec length

            assert_eq!(got[0], UnalignedStruct { a: 1 });
            assert_eq!(got[0].wire_size(), 1);

            assert_eq!(got[1], UnalignedStruct { a: 2 });
            assert_eq!(got[1].wire_size(), 1);

            assert_eq!(got[2], UnalignedStruct { a: 3 });
            assert_eq!(got[2].wire_size(), 1);

            assert_eq!(got[3], UnalignedStruct { a: 4 });
            assert_eq!(got[3].wire_size(), 1);

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

            let got = buf.read_variable_array::<UnalignedStruct>(None).unwrap();

            assert_eq!(got.len(), 2);
            assert_eq!(got.wire_size(), 4 + 4);
            assert_eq!(got[0], UnalignedStruct { a: 1 });
            assert_eq!(got[1], UnalignedStruct { a: 2 });

            assert_eq!(buf.len(), 4);
            assert_eq!(buf.as_ref(), &[0, 0, 0, 123]);
        }

        #[test]
        fn test_read_variable_bytes_no_max() {
            let mut buf = BytesMut::new();
            buf.put_u32(8); // Len=8
            buf.put([1, 2, 3, 4, 5, 6, 7, 8].as_ref());
            let mut buf = buf.freeze();

            let got = buf.read_variable_bytes(None).unwrap();

            assert_eq!(got.len(), 8);
            assert_eq!(got.wire_size(), 8);
            assert_eq!(got.as_ref(), &[1, 2, 3, 4, 5, 6, 7, 8]);

            assert_eq!(buf.as_ref(), &[]);
            assert_eq!(buf.remaining(), 0);
        }

        #[test]
        fn test_read_variable_bytes_no_max_with_padding() {
            let mut buf = BytesMut::new();
            buf.put_u32(6); // Len=6 + 2 bytes padding
            buf.put([1, 2, 3, 4, 5, 6, 0, 0].as_ref());
            let mut buf = buf.freeze();

            let got = buf.read_variable_bytes(None).unwrap();

            assert_eq!(got.len(), 6);
            assert_eq!(got.wire_size(), 6);
            assert_eq!(got.as_ref(), &[1, 2, 3, 4, 5, 6]);

            assert_eq!(buf.as_ref(), &[]);
            assert_eq!(buf.remaining(), 0);
        }

        #[test]
        fn test_read_bool() {
            let mut buf = BytesMut::new();
            buf.put_u32(0);
            buf.put_u32(1);
            buf.put_u32(2);
            let mut buf = buf.freeze();

            assert_eq!(buf.read_bool(), Ok(false));
            assert_eq!(buf.read_bool(), Ok(true));
            assert!(buf.read_bool().is_err());
        }
    }