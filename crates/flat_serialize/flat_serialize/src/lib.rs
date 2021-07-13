
use std::{fmt, marker::PhantomData, mem::{MaybeUninit, align_of, size_of}, slice};
#[derive(Debug)]
pub enum WrapErr {
    NotEnoughBytes(usize),
    InvalidTag(usize),
}

/// For a type to be `FlatSerializable` it must contain no pointers, have no
/// interior padding, must have a `size >= alignmen` and must have
/// `size % align = 0`. Use `#[derive(FlatSerializable)]` to implement this.
pub unsafe trait FlatSerializable<'input>: Sized + 'input {
    const MIN_LEN: usize;
    const REQUIRED_ALIGNMENT: usize;
    const MAX_PROVIDED_ALIGNMENT: Option<usize>;
    const TRIVIAL_COPY: bool = false;
    type SLICE;


    unsafe fn try_ref(input: &'input [u8]) -> Result<(Self, &'input [u8]), WrapErr>;
    fn fill_vec(&self, input: &mut Vec<u8>) {
        let start = input.len();
        let my_len = self.len();
        input.reserve(my_len);
        // simulate unstable spare_capacity_mut()
        let slice = unsafe {
            slice::from_raw_parts_mut(
                input.as_mut_ptr().add(input.len()) as *mut MaybeUninit<u8>,
                my_len,
            )
        };
        let rem = unsafe {
            self.fill_slice(slice)
        };
        debug_assert_eq!(rem.len(), 0);
        unsafe {
            input.set_len(start + my_len);
        }
    }
    #[must_use]
    unsafe fn fill_slice<'out>(&self, input: &'out mut [MaybeUninit<u8>])
    -> &'out mut [MaybeUninit<u8>];
    fn len(&self) -> usize;
}

#[macro_export]
macro_rules! impl_flat_serializable {
    ($($typ:ty)+) => {
        $(
            unsafe impl<'i> FlatSerializable<'i> for $typ {
                const MIN_LEN: usize = size_of::<Self>();
                const REQUIRED_ALIGNMENT: usize = align_of::<Self>();
                const MAX_PROVIDED_ALIGNMENT: Option<usize> = None;
                const TRIVIAL_COPY: bool = true;
                type SLICE = &'i [$typ];

                #[inline(always)]
                unsafe fn try_ref(input: &'i [u8])
                -> Result<(Self, &'i [u8]), WrapErr> {
                    let size = size_of::<Self>();
                    if input.len() < size {
                        return Err(WrapErr::NotEnoughBytes(size))
                    }
                    let (field, rem) = input.split_at(size);
                    let field = field.as_ptr().cast::<Self>();
                    Ok((field.read_unaligned(), rem))
                }

                #[inline(always)]
                unsafe fn fill_slice<'out>(&self, input: &'out mut [MaybeUninit<u8>])
                -> &'out mut [MaybeUninit<u8>] {
                    let size = size_of::<Self>();
                    let (input, rem) = input.split_at_mut(size);
                    let bytes = (self as *const Self).cast::<MaybeUninit<u8>>();
                    let bytes = slice::from_raw_parts(bytes, size);
                    // emulate write_slice_cloned()
                    // for i in 0..size {
                    //     input[i] = MaybeUninit::new(bytes[i])
                    // }
                    input.copy_from_slice(bytes);
                    rem
                }

                #[inline(always)]
                fn len(&self) -> usize {
                    size_of::<Self>()
                }
            }
        )+
    };
}

impl_flat_serializable!(bool);
impl_flat_serializable!(i8 u8 i16 u16 i32 u32 i64 u64 i128 u128);
impl_flat_serializable!(f32 f64 ordered_float::OrderedFloat<f32> ordered_float::OrderedFloat<f64>);

// TODO ensure perf
unsafe impl<'i, T, const N: usize> FlatSerializable<'i> for [T; N]
where T: FlatSerializable<'i> + 'i {
    const MIN_LEN: usize = {T::MIN_LEN * N};
    const REQUIRED_ALIGNMENT: usize = T::REQUIRED_ALIGNMENT;
    const MAX_PROVIDED_ALIGNMENT: Option<usize> = T::MAX_PROVIDED_ALIGNMENT;
    const TRIVIAL_COPY: bool = T::TRIVIAL_COPY;
    // FIXME ensure no padding
    type SLICE = Iterable<'i, [T; N]>;

    #[inline(always)]
    unsafe fn try_ref(mut input: &'i [u8])
    -> Result<(Self, &'i [u8]), WrapErr> {
        // TODO can we simplify based on T::TRIVIAL_COPY?
        if T::TRIVIAL_COPY {
            if input.len() < (T::MIN_LEN * N) {
                return Err(WrapErr::NotEnoughBytes(T::MIN_LEN * N))
            }
        }
        let mut output: [MaybeUninit<T>; N] = MaybeUninit::uninit().assume_init();
        for i in 0..N {
            let (val, rem) = T::try_ref(input)?;
            output[i] = MaybeUninit::new(val);
            input = rem;
        }
        let output = (&output as * const [MaybeUninit<T>; N])
            .cast::<[T; N]>().read();
        Ok((output, input))
    }

    #[inline(always)]
    unsafe fn fill_slice<'out>(&self, input: &'out mut [MaybeUninit<u8>])
    -> &'out mut [MaybeUninit<u8>] {
        let size = if Self::TRIVIAL_COPY {
            Self::MIN_LEN
        } else {
            self.len()
        };
        let (mut input, rem) = input.split_at_mut(size);
        input = &mut input[..size];
        // TODO is there a way to force a memcopy for trivial cases?

        for val in self {
            input = val.fill_slice(input);
        }
        debug_assert_eq!(input.len(), 0);
        rem
    }

    #[inline(always)]
    fn len(&self) -> usize {
        self.iter().map(T::len).sum()
    }
}


pub enum Iterable<'input, T: 'input,> {
    Iter(Iter<'input, T>),
    Slice(&'input [T]),
    Owned(Vec<T>),
}

impl <'input, T: 'input> Iterable<'input, T> {
    pub fn iter<'s>(&'s self) -> I<'s, T>
    where 'input: 's {
        match self {
            Iterable::Iter(iter) => I::Iter(*iter),
            Iterable::Slice(slice) => I::Slice(slice),
            Iterable::Owned(vec) => I::Slice(&*vec),
        }
    }
}

pub enum I<'input, T: 'input,> {
    Iter(Iter<'input, T>),
    Slice(&'input [T]),
}

impl<'input, T: 'input> Iterator for I<'input, T>
where T: FlatSerializable<'input> + Clone {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Iter(i) => {
                if i.slice.is_empty() {
                    return None
                }
                let (val, rem) = unsafe {
                    <T>::try_ref(i.slice).unwrap()
                };
                let additional_len = aligning_len(rem.as_ptr() as _, T::REQUIRED_ALIGNMENT);

                i.slice = &rem[additional_len..];
                return Some(val)
            },
            Self::Slice(s) => {
                let val = s.first().cloned();
                if val.is_some() {
                    *s = &s[1..]
                }
                val
            },
        }

    }
}

impl<'i, T: 'i> fmt::Debug for Iterable<'i, T>
where T: fmt::Debug + for<'s> FlatSerializable<'s> + Clone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<'i, T: 'i> PartialEq for Iterable<'i, T>
where T: for<'s> FlatSerializable<'s> + Clone + PartialEq {
    fn eq(&self, other: &Self) -> bool {
        <I<'_, T> as Iterator>::eq(self.iter(), other.iter())
    }
}

#[derive(Debug)]
pub struct Iter<'input, T: 'input> {
    slice: &'input [u8],
    _pd: PhantomData<&'input T>,
}

impl<'input, T: 'input> Iterable<'input, T> {
    pub unsafe fn from_bytes(bytes: &'input [u8]) -> Self {
        Iterable::Iter(Iter {
            slice: bytes,
            _pd: PhantomData,
        })
    }
}

impl<'input, T: 'input> Iterator for Iter<'input, T>
where T: FlatSerializable<'input> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.slice.is_empty() {
            return None
        }
        let (val, rem) = unsafe {
            <T>::try_ref(self.slice).unwrap()
        };
        self.slice = rem;
        return Some(val)
    }
}

impl<'input, T: 'input> From<&'input [T]> for Iterable<'input, T> {
    fn from(val: &'input [T]) -> Self {
        Self::Slice(val)
    }
}

impl<'input, T: 'input> Clone for Iterable<'input, T>
where T: Clone {
    fn clone(&self) -> Self {
        match self {
            Iterable::Iter(i) => Iterable::Iter(*i),
            Iterable::Slice(s) => Iterable::Slice(&*s),
            Iterable::Owned(v) => Iterable::Owned(Vec::clone(v)),
        }
    }
}

impl<'i, T> serde::Serialize for Iterable<'i, T>
where T: serde::Serialize + Clone + for<'t> FlatSerializable<'t> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        use serde::ser::SerializeSeq;

        // TODO len should be computable
        let mut s = serializer.serialize_seq(None)?;
        for t in self.iter() {
            s.serialize_element(&t)?
        }
        s.end()
    }
}

impl<'de, T> serde::Deserialize<'de> for Iterable<'static, T>
where T: serde::Deserialize<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> {
        let v = Vec::deserialize(deserializer)?;
        Ok(Self::Owned(v))
    }
}

impl<'input, T: 'input> Clone for Iter<'input, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'input, T: 'input> Copy for Iter<'input, T> {}

#[doc(hidden)]
pub unsafe trait Slice<'input>: Sized {
    unsafe fn try_ref(input: &'input [u8], count: usize) -> Result<(Self, &'input [u8]), WrapErr>;
    #[must_use]
    unsafe fn fill_slice<'out>(&self, count: usize, input: &'out mut [MaybeUninit<u8>])
    -> &'out mut [MaybeUninit<u8>];
    fn len(&self, count: usize) -> usize;
}

unsafe impl<'i, T: 'i> Slice<'i> for &'i [T]
where T: FlatSerializable<'i> {
    #[inline(always)]
    unsafe fn try_ref(input: &'i [u8], count: usize) -> Result<(Self, &'i [u8]), WrapErr> {
        assert!(<T as FlatSerializable>::TRIVIAL_COPY);
        let byte_len = T::MIN_LEN * count;
        if input.len() < byte_len {
            return Err(WrapErr::NotEnoughBytes(byte_len));
        }
        let (bytes, rem) = input.split_at(byte_len);
        let bytes = bytes.as_ptr();
        let field = ::std::slice::from_raw_parts(bytes.cast::<T>(), count);
        debug_assert_eq!(
            bytes.offset(byte_len as isize) as usize,
            field.as_ptr().offset(count as isize) as usize
        );
        Ok((field, rem))
    }

    #[inline(always)]
    unsafe fn fill_slice<'out>(&self, count: usize, input: &'out mut [MaybeUninit<u8>])
    -> &'out mut [MaybeUninit<u8>] {
        assert!(<T as FlatSerializable>::TRIVIAL_COPY);
        if !<T as FlatSerializable>::TRIVIAL_COPY {
            return fill_slice_from_iter::<T, _, _>(self.iter(), count, input)
        }
        let vals = &self[..count];
        let size = <T>::MIN_LEN * vals.len();
        let (out, rem) = input.split_at_mut(size);
        let bytes = vals.as_ptr().cast::<std::mem::MaybeUninit<u8>>();
        let bytes = std::slice::from_raw_parts(bytes, size);
        out.copy_from_slice(bytes);
        rem

    }

    #[inline(always)]
    fn len(&self, count: usize) -> usize {
        assert!(<T as FlatSerializable>::TRIVIAL_COPY);
        if !<T as FlatSerializable>::TRIVIAL_COPY {
            return len_of_iterable::<T, _, _>(self.iter(), count)
        }
        ::std::mem::size_of::<T>() * count as usize
    }
}

unsafe impl<'i, T: 'i> Slice<'i> for Iterable<'i, T>
where T: for<'s> FlatSerializable<'s> + Clone {
    #[inline(always)]
    unsafe fn try_ref(input: &'i [u8], count: usize) -> Result<(Self, &'i [u8]), WrapErr> {
        if T::TRIVIAL_COPY {
            let (field, rem) = <&[T]>::try_ref(input, count)?;
            return Ok((Self::Slice(field), rem))
        }
        let mut total_len = 0;
        let mut tmp = input;
        let mut old_ptr = input.as_ptr() as usize;
        for _ in 0..count {
            let (field, rem) = T::try_ref(tmp)?;
            debug_assert_eq!(rem.as_ptr() as usize - old_ptr, field.len());

            let additional_len = aligning_len(rem.as_ptr() as _, T::REQUIRED_ALIGNMENT);
            if rem.len() < additional_len {
                return Err(WrapErr::NotEnoughBytes(additional_len))
            }

            let rem = &rem[additional_len..];
            debug_assert_eq!(rem.as_ptr() as usize % T::REQUIRED_ALIGNMENT, 0);

            let padded_len = rem.as_ptr() as usize - old_ptr;

            old_ptr = rem.as_ptr() as usize;
            tmp = rem;
            total_len += padded_len;
        }
        let (iter, rem) = input.split_at(total_len);
        debug_assert_eq!(rem.as_ptr() as usize, tmp.as_ptr() as usize);
        debug_assert_eq!(rem.len(), tmp.len());
        Ok((
            Self::from_bytes(iter),
            rem
        ))
    }

    #[inline(always)]
    unsafe fn fill_slice<'out>(&self, count: usize, input: &'out mut [MaybeUninit<u8>])
    -> &'out mut [MaybeUninit<u8>] {
        if let (true, Self::Slice(values)) = (T::TRIVIAL_COPY, self) {
            return <&[T]>::fill_slice(values, count, input)
        }
        fill_slice_from_iter(self.iter(), count, input)
    }

    #[inline(always)]
    fn len(&self, count: usize) -> usize {
        if let (true, Self::Slice(values)) = (T::TRIVIAL_COPY, self) {
            return <&[T]>::len(values, count)
        }
        len_of_iterable(self.iter(), count)
    }
}

#[inline(always)]
unsafe fn fill_slice_from_iter<'i, 'out, T: FlatSerializable<'i>, V: ValOrRef<T>, I: Iterator<Item=V>>(
    iter: I,
    count: usize,
    mut input: &'out mut [MaybeUninit<u8>]
) -> &'out mut [MaybeUninit<u8>] {
    let mut filled = 0;
    for v in iter.take(count) {
        input = v.to_ref().fill_slice(input);
        let additional_len = aligning_len(input.as_ptr(), T::REQUIRED_ALIGNMENT);
        let (addition, rem) = input.split_at_mut(additional_len);
        addition.copy_from_slice(&[MaybeUninit::new(0); 8][..additional_len]);
        debug_assert_eq!(rem.as_ptr() as usize % T::REQUIRED_ALIGNMENT, 0);
        input = rem;
        filled += 1;
    }
    if filled < count {
        panic!("Not enough elements. Expected {} found {}", count, filled)
    }
    input
}

#[inline(always)]
fn len_of_iterable<'i, T: FlatSerializable<'i>, V: ValOrRef<T>, I: Iterator<Item=V>>(
    iter: I,
    count: usize,
) -> usize {
    let mut filled = 0;
    let mut len = 0;
    for v in iter.take(count) {
        filled += 1;
        len += v.to_ref().len();
        if len % T::REQUIRED_ALIGNMENT != 0 {
            len += T::REQUIRED_ALIGNMENT - (len % T::REQUIRED_ALIGNMENT);
        }
    }
    if filled < count {
        panic!("Not enough elements. Expected {} found {}", count, filled)
    }
    len
}

#[inline(always)]
fn aligning_len(ptr: *const MaybeUninit<u8>, align: usize) -> usize {
    let current_ptr = ptr as usize;
    if current_ptr as usize % align == 0 {
        return 0
    }
    align - (current_ptr % align)
}

trait ValOrRef<T: ?Sized> {
    fn to_ref(&self) -> &T;
}

impl<T: ?Sized> ValOrRef<T> for T {
    fn to_ref(&self) -> &T {
        self
    }
}

impl<T: ?Sized> ValOrRef<T> for &T {
    fn to_ref(&self) -> &T {
        *self
    }
}

#[cfg(test)]
mod tests {
    use crate as flat_serialize;

    use flat_serialize_macro::{flat_serialize, FlatSerializable};

    flat_serialize! {
        #[derive(Debug)]
        struct Basic<'input> {
            header: u64,
            data_len: u32,
            array: [u16; 3],
            data: [u8; self.data_len],
            data2: [[u8; 2]; self.data_len / 3],
        }
    }

    #[test]
    fn basic() {
        use crate::{FlatSerializable, WrapErr, Iterable};
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&33u64.to_ne_bytes());
        bytes.extend_from_slice(&6u32.to_ne_bytes());
        bytes.extend_from_slice(&202u16.to_ne_bytes());
        bytes.extend_from_slice(&404u16.to_ne_bytes());
        bytes.extend_from_slice(&555u16.to_ne_bytes());
        bytes.extend_from_slice(&[1, 3, 5, 7, 9, 11]);
        bytes.extend_from_slice(&[4, 4, 95, 99]);
        let (
            Basic {
                header,
                data_len,
                data,
                data2,
                array,
            },
            rem,
        ) = unsafe { Basic::try_ref(&bytes).unwrap() };
        assert_eq!(
            (header, data_len, array, data, &data2, rem),
            (
                33,
                6,
                [202, 404, 555],
                &[1, 3, 5, 7, 9, 11][..],
                &Iterable::Slice(&[[4, 4], [95, 99]]),
                &[][..]
            )
        );


        let mut output = vec![];
        Basic {
            header,
            data_len,
            data,
            data2: data2.clone(),
            array,
        }
        .fill_vec(&mut output);
        assert_eq!(output, bytes);

        let debug = format!(
            "{:?}",
            Basic {
                header,
                data_len,
                data,
                data2,
                array
            }
        );
        assert_eq!(debug, "Basic { header: 33, data_len: 6, array: [202, 404, 555], data: [1, 3, 5, 7, 9, 11], data2: [[4, 4], [95, 99]] }");

        assert_eq!(Basic::MIN_LEN, 18);
        assert_eq!(Basic::REQUIRED_ALIGNMENT, 8);
        assert_eq!(Basic::MAX_PROVIDED_ALIGNMENT, Some(1));
        assert_eq!(Basic::TRIVIAL_COPY, false);

        for i in 0..bytes.len()-1 {
            let res = unsafe {
                Basic::try_ref(&bytes[..i])
            };
            assert!(matches!(res, Err(WrapErr::NotEnoughBytes(..))), "{:?}", res);
        }
    }

    #[test]
    #[should_panic(expected = "range end index 5 out of range for slice of length 1")]
    fn bad_len1() {
        use crate::{FlatSerializable, Iterable};
        let mut output = vec![];
        Basic {
            header: 1,
            data_len: 5,
            array: [0; 3],
            data: &[1],
            data2: Iterable::Slice(&[[2, 2]]),
        }
        .fill_vec(&mut output);
    }

    #[test]
    #[should_panic(expected = "range end index 1 out of range for slice of length 0")]
    fn bad_len2() {
        use crate::{FlatSerializable, Iterable};
        let mut output = vec![];
        Basic {
            header: 1,
            data_len: 5,
            array: [0; 3],
            data: &[1, 2, 3, 4, 5],
            data2: Iterable::Slice(&[]),
        }
        .fill_vec(&mut output);
    }

    flat_serialize! {
        #[derive(Debug, PartialEq, Eq)]
        struct Optional {
            header: u64,
            optional_field: u32 if self.header != 1,
            non_optional_field: u16,
        }
    }

    const _TEST_NO_VARIABLE_LEN_NO_LIFETIME: Optional = Optional {
        header: 0,
        optional_field: None,
        non_optional_field: 0,
    };

    #[test]
    fn optional_present() {
        use crate::{FlatSerializable, WrapErr};
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&101010101u64.to_ne_bytes());
        bytes.extend_from_slice(&30u32.to_ne_bytes());
        bytes.extend_from_slice(&6u16.to_ne_bytes());
        let (
            Optional {
                header,
                optional_field,
                non_optional_field,
            },
            rem,
        ) = unsafe { Optional::try_ref(&bytes).unwrap() };
        assert_eq!(
            (header, optional_field, non_optional_field, rem),
            (
                101010101,
                Some(30),
                6,
                &[][..]
            )
        );

        let mut output = vec![];
        Optional {
            header,
            optional_field,
            non_optional_field,
        }
        .fill_vec(&mut output);
        assert_eq!(output, bytes);

        for i in 0..bytes.len()-1 {
            let res = unsafe {
                Optional::try_ref(&bytes[..i])
            };
            assert!(matches!(res, Err(WrapErr::NotEnoughBytes(..))), "{:?}", res);
        }

        assert_eq!(Optional::MIN_LEN, 10);
        assert_eq!(Optional::REQUIRED_ALIGNMENT, 8);
        assert_eq!(Optional::MAX_PROVIDED_ALIGNMENT, Some(2));
        assert_eq!(Optional::TRIVIAL_COPY, false);
    }

    #[test]
    fn optional_absent() {
        use crate::{FlatSerializable, WrapErr};
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&1u64.to_ne_bytes());
        bytes.extend_from_slice(&7u16.to_ne_bytes());
        let (
            Optional {
                header,
                optional_field,
                non_optional_field,
            },
            rem,
        ) = unsafe { Optional::try_ref(&bytes).unwrap() };
        assert_eq!(
            (header, optional_field, non_optional_field, rem),
            (
                1,
                None,
                7,
                &[][..]
            )
        );

        let mut output = vec![];
        Optional {
            header,
            optional_field,
            non_optional_field,
        }
        .fill_vec(&mut output);
        assert_eq!(output, bytes);

        for i in 0..bytes.len()-1 {
            let res = unsafe {
                Optional::try_ref(&bytes[..i])
            };
            assert!(matches!(res, Err(WrapErr::NotEnoughBytes(..))), "{:?}", res);
        }
    }

    flat_serialize! {
        #[derive(Debug)]
        struct Nested<'a> {
            prefix: u64,
            basic: Basic<'a>,
        }
    }

    #[test]
    fn nested() {
        use crate::{FlatSerializable, WrapErr, Iterable};
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&101010101u64.to_ne_bytes());
        bytes.extend_from_slice(&33u64.to_ne_bytes());
        bytes.extend_from_slice(&6u32.to_ne_bytes());
        bytes.extend_from_slice(&202u16.to_ne_bytes());
        bytes.extend_from_slice(&404u16.to_ne_bytes());
        bytes.extend_from_slice(&555u16.to_ne_bytes());
        bytes.extend_from_slice(&[1, 3, 5, 7, 9, 11]);
        bytes.extend_from_slice(&[3, 0, 104, 2]);
        let (
            Nested {
                prefix,
                basic:
                    Basic {
                        header,
                        data_len,
                        array,
                        data,
                        data2,
                    },
            },
            rem,
        ) = unsafe { Nested::try_ref(&bytes).unwrap() };
        assert_eq!(
            (prefix, header, data_len, array, data, &data2, rem),
            (
                101010101,
                33,
                6,
                [202, 404, 555],
                &[1, 3, 5, 7, 9, 11][..],
                &Iterable::Slice(&[[3, 0], [104, 2]]),
                &[][..]
            )
        );

        let mut output = vec![];
        Nested {
            prefix,
            basic: Basic {
                header,
                data_len,
                data,
                data2,
                array,
            },
        }
        .fill_vec(&mut output);
        assert_eq!(output, bytes);

        for i in 0..bytes.len()-1 {
            let res = unsafe {
                Nested::try_ref(&bytes[..i])
            };
            assert!(matches!(res, Err(WrapErr::NotEnoughBytes(..))), "{:?}", res);
        }
    }

    flat_serialize!{
        #[derive(Debug)]
        struct NestedOptional {
            present: u64,
            val: Optional if self.present > 2,
        }
    }

    #[test]
    fn nested_optional() {
        use crate::{FlatSerializable, WrapErr};
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&3u64.to_ne_bytes());
        {
            bytes.extend_from_slice(&0u64.to_ne_bytes());
            bytes.extend_from_slice(&111111111u32.to_ne_bytes());
            bytes.extend_from_slice(&0xf00fu16.to_ne_bytes());
            bytes.extend_from_slice(&[77; 2]);
        }

        let (NestedOptional { present, val }, rem) = unsafe {
            NestedOptional::try_ref(&bytes).unwrap()
        };

        assert_eq!(
            (present, &val, rem),
            (
                3,
                &Some(Optional {
                    header: 0,
                    optional_field: Some(111111111),
                    non_optional_field: 0xf00f,
                }),
                &[77; 2][..],
            )
        );

        let mut output = vec![];
        NestedOptional { present, val }.fill_vec(&mut output);
        assert_eq!(output, &bytes[..bytes.len()-2]);

        for i in 0..bytes.len()-3 {
            let res = unsafe {
                NestedOptional::try_ref(&bytes[..i])
            };
            assert!(matches!(res, Err(WrapErr::NotEnoughBytes(..))), "{:?}", res);
        }

        assert_eq!(NestedOptional::MIN_LEN, 8);
        assert_eq!(NestedOptional::REQUIRED_ALIGNMENT, 8);
        assert_eq!(NestedOptional::MAX_PROVIDED_ALIGNMENT, Some(2));
        assert_eq!(NestedOptional::TRIVIAL_COPY, false);
    }

    flat_serialize!{
        #[derive(Debug)]
        struct NestedSlice<'b> {
            num_vals: u64,
            // #[flat_serialize::flatten]
            vals: [Optional; self.num_vals],
        }
    }

    #[test]
    fn nested_slice() {
        use crate::{FlatSerializable, WrapErr};
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&3u64.to_ne_bytes());
        {
            bytes.extend_from_slice(&101010101u64.to_ne_bytes());
            bytes.extend_from_slice(&30u32.to_ne_bytes());
            bytes.extend_from_slice(&6u16.to_ne_bytes());
            bytes.extend_from_slice(&[0; 2]);
        }
        {
            bytes.extend_from_slice(&1u64.to_ne_bytes());
            bytes.extend_from_slice(&7u16.to_ne_bytes());
            bytes.extend_from_slice(&[0; 6]);
        }
        {
            bytes.extend_from_slice(&0u64.to_ne_bytes());
            bytes.extend_from_slice(&111111111u32.to_ne_bytes());
            bytes.extend_from_slice(&0xf00fu16.to_ne_bytes());
            bytes.extend_from_slice(&[0; 2]);
        }

        let (NestedSlice { num_vals, vals }, rem) = unsafe {
            NestedSlice::try_ref(&bytes).unwrap()
        };
        let vals_vec: Vec<_> = vals.iter().collect();
        assert_eq!(
            (num_vals, &*vals_vec, rem),
            (
                3,
                &[
                    Optional {
                        header: 101010101,
                        optional_field: Some(30),
                        non_optional_field: 6,
                    },
                    Optional {
                        header: 1,
                        optional_field: None,
                        non_optional_field: 7,
                    },
                    Optional {
                        header: 0,
                        optional_field: Some(111111111),
                        non_optional_field: 0xf00f,
                    },
                ][..],
                &[][..],
            )
        );

        let mut output = vec![];
        NestedSlice { num_vals, vals }.fill_vec(&mut output);
        assert_eq!(output, bytes);

        for i in 0..bytes.len()-1 {
            let res = unsafe {
                NestedSlice::try_ref(&bytes[..i])
            };
            assert!(matches!(res, Err(WrapErr::NotEnoughBytes(..))), "{:?}", res);
        }

        assert_eq!(NestedSlice::MIN_LEN, 8);
        assert_eq!(NestedSlice::REQUIRED_ALIGNMENT, 8);
        assert_eq!(NestedSlice::MAX_PROVIDED_ALIGNMENT, Some(8));
        assert_eq!(NestedSlice::TRIVIAL_COPY, false);
    }

    flat_serialize! {
        #[derive(Debug)]
        enum BasicEnum<'input> {
            k: u64,
            First: 2 {
                data_len: u32,
                data: [u8; self.data_len],
            },
            Fixed: 3 {
                array: [u16; 3],
            },
        }
    }

    #[test]
    fn basic_enum1() {
        use crate::{FlatSerializable, WrapErr};
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&2u64.to_ne_bytes());
        bytes.extend_from_slice(&6u32.to_ne_bytes());
        bytes.extend_from_slice(&[1, 3, 5, 7, 9, 11]);
        let (data_len, data, rem) = match unsafe { BasicEnum::try_ref(&bytes).unwrap() } {
            (BasicEnum::First { data_len, data }, rem) => (data_len, data, rem),
            _ => unreachable!(),
        };
        assert_eq!(
            (data_len, data, rem),
            (6, &[1, 3, 5, 7, 9, 11][..], &[][..])
        );

        let mut output = vec![];
        BasicEnum::First { data_len, data }.fill_vec(&mut output);
        assert_eq!(output, bytes);

        for i in 0..bytes.len()-1 {
            let res = unsafe {
                BasicEnum::try_ref(&bytes[..i])
            };
            assert!(matches!(res, Err(WrapErr::NotEnoughBytes(..))), "{:?}", res);
        }
    }

    #[test]
    fn basic_enum2() {
        use crate::{FlatSerializable, WrapErr};
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&3u64.to_ne_bytes());
        bytes.extend_from_slice(&3u16.to_ne_bytes());
        bytes.extend_from_slice(&6u16.to_ne_bytes());
        bytes.extend_from_slice(&9u16.to_ne_bytes());
        bytes.extend_from_slice(&[7]);
        let (array, rem) = match unsafe { BasicEnum::try_ref(&bytes).unwrap() } {
            (BasicEnum::Fixed { array }, rem) => (array, rem),
            _ => unreachable!(),
        };
        assert_eq!((array, rem), ([3, 6, 9], &[7][..]));

        let (array, rem) = match unsafe { BasicEnum::try_ref(&bytes).unwrap() } {
            (BasicEnum::Fixed { array }, rem) => (array, rem),
            _ => unreachable!(),
        };
        assert_eq!((array, rem), ([3, 6, 9], &[7][..]));

        let mut output = vec![];
        BasicEnum::Fixed { array }.fill_vec(&mut output);
        assert_eq!(output, &bytes[..bytes.len() - 1]);

        for i in 0..bytes.len()-1 {
            let res = unsafe {
                BasicEnum::try_ref(&bytes[..i])
            };
            assert!(matches!(res, Err(WrapErr::NotEnoughBytes(..))), "{:?}", res);
        }
    }



    flat_serialize! {
        #[derive(Debug)]
        enum PaddedEnum<'input> {
            k: u8,
            First: 2 {
                padding: [u8; 3],
                data_len: u32,
                data: [u8; self.data_len],
            },
            Fixed: 3 {
                padding: u8,
                array: [u16; 3],
            },
        }
    }

    #[test]
    fn padded_enum1() {
        use crate::{FlatSerializable, WrapErr};
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&2u8.to_ne_bytes());
        bytes.extend_from_slice(&[0xf, 0xf, 0xf]);
        bytes.extend_from_slice(&6u32.to_ne_bytes());
        bytes.extend_from_slice(&[1, 3, 5, 7, 9, 11]);
        let (padding, data_len, data, rem) = match unsafe { PaddedEnum::try_ref(&bytes).unwrap() } {
            (PaddedEnum::First {  padding, data_len, data }, rem) => (padding, data_len, data, rem),
            _ => unreachable!(),
        };
        assert_eq!(
            (padding, data_len, data, rem),
            ([0xf, 0xf, 0xf], 6, &[1, 3, 5, 7, 9, 11][..], &[][..])
        );

        let mut output = vec![];
        PaddedEnum::First { padding, data_len, data }.fill_vec(&mut output);
        assert_eq!(output, bytes);

        for i in 0..bytes.len()-1 {
            let res = unsafe {
                PaddedEnum::try_ref(&bytes[..i])
            };
            assert!(matches!(res, Err(WrapErr::NotEnoughBytes(..))), "{:?}", res);
        }
    }

    #[test]
    fn padded_enum2() {
        use crate::{FlatSerializable, WrapErr};
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&3u8.to_ne_bytes());
        bytes.extend_from_slice(&[0]);
        bytes.extend_from_slice(&3u16.to_ne_bytes());
        bytes.extend_from_slice(&6u16.to_ne_bytes());
        bytes.extend_from_slice(&9u16.to_ne_bytes());
        bytes.extend_from_slice(&[7]);
        let (padding, array, rem) = match unsafe { PaddedEnum::try_ref(&bytes).unwrap() } {
            (PaddedEnum::Fixed { padding, array }, rem) => (padding, array, rem),
            _ => unreachable!(),
        };
        assert_eq!((padding, array, rem), (0, [3, 6, 9], &[7][..]));

        let (padding, array, rem) = match unsafe { PaddedEnum::try_ref(&bytes).unwrap() } {
            (PaddedEnum::Fixed {padding, array }, rem) => (padding, array, rem),
            _ => unreachable!(),
        };
        assert_eq!((padding, array, rem), (0, [3, 6, 9], &[7][..]));

        let mut output = vec![];
        PaddedEnum::Fixed { padding, array }.fill_vec(&mut output);
        assert_eq!(output, &bytes[..bytes.len() - 1]);

        for i in 0..bytes.len()-1 {
            let res = unsafe {
                PaddedEnum::try_ref(&bytes[..i])
            };
            assert!(matches!(res, Err(WrapErr::NotEnoughBytes(..))), "{:?}", res);
        }
    }


    macro_rules! sub_macro {
        (
            $(#[$attrs: meta])?
            struct $name: ident {
                $($field:ident : $typ: tt),*
                $(,)?
            }
        ) => {
            flat_serialize_macro::flat_serialize! {
                $(#[$attrs])?
                struct $name {
                    $($field: $typ),*
                }
            }
        }
    }

    // test that sub_macros provide correct compilation
    sub_macro! {
        #[derive(Debug)]
        struct InMacro {
            a: u32,
            padding: [u8; 4], // with this commented out, the error should be on b
            b: f64,
        }
    }

    #[test]
    fn test_no_refrence() {
        flat_serialize!{
            struct NoLifetime {
                val: i64,
            }
        }

        let _: NoLifetime = NoLifetime{ val: 3 };

        flat_serialize!{
            struct NestedNoLifetime {
                nested: NoLifetime,
            }
        }

        let _: NestedNoLifetime = NestedNoLifetime{ nested: NoLifetime{ val: 3 } };

        flat_serialize!{
            enum ENoLifetime {
                tag: i64,
                Variant: 1 {
                    val: i64,
                },
            }
        }

        let _: ENoLifetime = ENoLifetime::Variant{ val: 2 };

        flat_serialize!{
            enum NestedENoLifetime {
                tag: i64,
                Variant: 2 {
                    val: ENoLifetime,
                },
            }
        }

        let _: NestedENoLifetime = NestedENoLifetime::Variant{val: ENoLifetime::Variant{ val: 2 }};
    }

    macro_rules! check_size_align {
        (struct $($dec_life:lifetime)? {
            $( $(#[$attrs: meta])*  $field:ident : $typ: tt $(<$life:lifetime>)?),*
            $(,)?
        }
            len: $min_len: expr,
            align: $required_alignment: expr,
            max: $max_provided_alignment: expr $(,)?
        ) => {
            {
                flat_serialize!{
                    struct SizeAlignTest $(<$dec_life>)? {
                        $($(#[$attrs])* $field: $typ $(<$life>)?),*
                    }
                };
                assert_eq!(<SizeAlignTest as crate::FlatSerializable>::MIN_LEN, $min_len, "length");
                assert_eq!(<SizeAlignTest as crate::FlatSerializable>::REQUIRED_ALIGNMENT, $required_alignment, "required");
                assert_eq!(<SizeAlignTest as crate::FlatSerializable>::MAX_PROVIDED_ALIGNMENT, $max_provided_alignment, "max provided");
                assert_eq!(<SizeAlignTest as crate::FlatSerializable>::TRIVIAL_COPY, false, "trivial copy");
            }
        }
    }

    #[test]
    fn test_size_align_struct() {
        check_size_align!(
            struct {
                f: u8,
            }
            len: 1,
            align: 1,
            max: None,
        );


        check_size_align!(
            struct {
                f: u16,
            }
            len: 2,
            align: 2,
            max: None,
        );

        check_size_align!(
            struct {
                f: u32,
            }
            len: 4,
            align: 4,
            max: None,
        );

        check_size_align!(
            struct {
                f: u64,
            }
            len: 8,
            align: 8,
            max: None,
        );

        check_size_align!(
            struct {
                a: u64,
                b: u32,
                c: u16,
            }
            len: 8 + 4 + 2,
            align: 8,
            max: None,
        );

        check_size_align!(
            struct {
                a: u32,
                b: u32,
                c: u32,
            }
            len: 4 + 4 + 4,
            align: 4,
            max: None,
        );

        check_size_align!(
            struct {
                a: [u32; 3],
            }
            len: 4 * 3,
            align: 4,
            max: None,
        );

        check_size_align!(
            struct 'a {
                a: u32,
                b: [u16; self.a],
            }
            len: 4,
            align: 4,
            max: Some(2),
        );

        check_size_align!(
            struct 'a {
                a: u32,
                b: [u32; self.a],
            }
            len: 4,
            align: 4,
            max: Some(4),
        );

        check_size_align!(
            struct 'a {
                a: u32,
                b: [u32; self.a],
                c: u32,
            }
            len: 4 + 4,
            align: 4,
            max: Some(4),
        );

        flat_serialize!{
            struct NestedA {
                a: u32,
                b: u16,
            }
        }

        check_size_align!(
            struct {
                a: u32,
                b: NestedA,
            }
            len: 4 + (4 + 2),
            align: 4,
            max: None,
        );

        check_size_align!(
            struct {
                a: u64,
                b: NestedA,
            }
            len: 8 + (4 + 2),
            align: 8,
            max: None,
        );

        check_size_align!(
            struct {
                a: u64,
                b: NestedA,
                c: u8
            }
            len: 8 + (4 + 2) + 1,
            align: 8,
            max: None,
        );

        check_size_align!(
            struct {
                a: NestedA,
                b: u8,
                c: u8,
                f: NestedA,
            }
            len: (4 + 2) + 1 + 1 + (4 + 2),
            align: 4,
            max: None,
        );

        flat_serialize!{
            struct NestedB<'input> {
                a: u32,
                b: [u16; self.a],
            }
        }

        check_size_align!(
            struct 'a {
                a: u32,
                b: NestedB<'a>,
            }
            len: 4 + (4),
            align: 4,
            max: Some(2),
        );

        check_size_align!(
            struct 'a {
                a: u64,
                b: NestedB<'a>,
            }
            len: 8 + (4),
            align: 8,
            max: Some(2),
        );

        check_size_align!(
            struct 'a {
                a: u64,
                b: NestedB<'a>,
                c: u8
            }
            len: 8 + (4) + 1,
            align: 8,
            max: Some(1),
        );

        check_size_align!(
            struct 'a {
                a: u8,
                b: u8,
                c: u8,
                d: u8,
                e: NestedB<'a>,
            }
            len: 4 + (4),
            align: 4,
            max: Some(2),
        );
    }

    #[test]
    fn test_size_align_enum() {

        flat_serialize!{
            enum EnumA {
                tag: u32,
                A: 1 {
                    a: u32,
                },
                B: 2 {
                    a: u16,
                },
            }
        }

        check_size_align!(
            struct {
                a: EnumA,
            }
            len: (4 + 2),
            align: 4,
            max: Some(2),
        );

        check_size_align!(
            struct {
                a: EnumA,
                b: u16,
            }
            len: (4 + 2) + 2,
            align: 4,
            max: Some(2),
        );

        check_size_align!(
            struct {
                b: u64,
                a: EnumA,
            }
            len: 8 + (4 + 2),
            align: 8,
            max: Some(2),
        );

        flat_serialize!{
            enum EnumB {
                tag: u32,
                A: 1 {
                    a: [u8; 5],
                },
                B: 2 {
                    a: u16,
                },
            }
        }

        check_size_align!(
            struct {
                a: EnumB,
            }
            len: (4 + 2),
            align: 4,
            max: Some(1),
        );

        check_size_align!(
            struct {
                b: u64,
                a: EnumB,
            }
            len: 8 + (4 + 2),
            align: 8,
            max: Some(1),
        );

        flat_serialize!{
            enum EnumC<'input> {
                tag: u64,
                A: 1 {
                    a: u64,
                },
                B: 2 {
                    a: u16,
                    b: [u16; self.a],
                },
            }
        }

        check_size_align!(
            struct 'a {
                a: EnumC<'a>,
            }
            len: (8 + 2),
            align: 8,
            max: Some(2),
        );

        check_size_align!(
            struct 'a {
                a: EnumC<'a>,
                b: u16,
            }
            len: (8 + 2) + 2,
            align: 8,
            max: Some(2),
        );

        check_size_align!(
            struct 'a {
                b: u64,
                a: EnumC<'a>,
            }
            len: 8 + (8 + 2),
            align: 8,
            max: Some(2),
        );

        flat_serialize!{
            enum EnumD<'input> {
                tag: u32,
                A: 1 {
                    a: u16,
                },
                B: 2 {
                    a: u32,
                    b: [u8; self.a],
                },
            }
        }

        check_size_align!(
            struct 'a {
                a: EnumD<'a>,
            }
            len: (4 + 2),
            align: 4,
            max: Some(1),
        );

        check_size_align!(
            struct 'a {
                a: EnumD<'a>,
                b: u8,
            }
            len: (4 + 2) + 1,
            align: 4,
            max: Some(1),
        );

        check_size_align!(
            struct 'a {
                b: u64,
                a: EnumD<'a>,
            }
            len: 8 + (4 + 2),
            align: 8,
            max: Some(1),
        );
    }

    #[derive(FlatSerializable)]
    #[allow(dead_code)]
    #[derive(Debug)]
    struct Foo {
        a: i32,
        b: i32,
    }

    const _:() = {
        fn check_flat_serializable_impl<'a, T: crate::FlatSerializable<'a>>() {}
        let _ = check_flat_serializable_impl::<Foo>;
        let _ = check_flat_serializable_impl::<[Foo; 2]>;
    };

    #[test]
    fn foo() {
        use crate::{FlatSerializable, WrapErr};
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&33i32.to_ne_bytes());
        bytes.extend_from_slice(&100000001i32.to_ne_bytes());

        let (Foo {a, b}, rem) = unsafe {
            Foo::try_ref(&bytes).unwrap()
        };
        assert_eq!(
            (a, b, rem),
            (33, 100000001, &[][..]),
        );

        let mut output = vec![];
        Foo { a, b }.fill_vec(&mut output);
        assert_eq!(output, bytes);

        assert_eq!(Foo::MIN_LEN, 8);
        assert_eq!(Foo::REQUIRED_ALIGNMENT, 4);
        assert_eq!(Foo::MAX_PROVIDED_ALIGNMENT, None);
        assert_eq!(Foo::TRIVIAL_COPY, true);

        for i in 0..bytes.len()-1 {
            let res = unsafe {
                Foo::try_ref(&bytes[..i])
            };
            assert!(matches!(res, Err(WrapErr::NotEnoughBytes(..))), "{:?}", res);
        }
    }

    #[derive(FlatSerializable)]
    #[allow(dead_code)]
    #[repr(u16)]
    #[derive(Debug, Copy, Clone)]
    enum Bar {
        A = 0,
        B = 1111,
    }

    const _:() = {
        fn check_flat_serializable_impl<'a, T: crate::FlatSerializable<'a>>() {}
        let _ = check_flat_serializable_impl::<Bar>;
        let _ = check_flat_serializable_impl::<[Bar; 2]>;
    };

    #[test]
    fn fs_enum_a() {
        use crate::{FlatSerializable, WrapErr};
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0u16.to_ne_bytes());

        let (
            val,
            rem,
        ) = unsafe { Bar::try_ref(&bytes).unwrap() };
        assert_eq!(
            (val as u16, rem),
            (Bar::A as u16, &[][..])
        );

        let mut output = vec![];
        val.fill_vec(&mut output);
        assert_eq!(output, bytes);

        assert_eq!(Bar::MIN_LEN, 2);
        assert_eq!(Bar::REQUIRED_ALIGNMENT, 2);
        assert_eq!(Bar::MAX_PROVIDED_ALIGNMENT, None);
        assert_eq!(Bar::TRIVIAL_COPY, true);

        for i in 0..bytes.len()-1 {
            let res = unsafe {
                Bar::try_ref(&bytes[..i])
            };
            assert!(matches!(res, Err(WrapErr::NotEnoughBytes(..))), "{:?}", res);
        }
    }

    #[test]
    fn fs_enum_b() {
        use crate::{FlatSerializable, WrapErr};
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&1111u16.to_ne_bytes());

        let (
            val,
            rem,
        ) = unsafe { Bar::try_ref(&bytes).unwrap() };
        assert_eq!(
            (val as u16, rem),
            (Bar::B as u16, &[][..])
        );

        let mut output = vec![];
        val.fill_vec(&mut output);
        assert_eq!(output, bytes);

        for i in 0..bytes.len()-1 {
            let res = unsafe {
                Bar::try_ref(&bytes[..i])
            };
            assert!(matches!(res, Err(WrapErr::NotEnoughBytes(..))), "{:?}", res);
        }
    }

    #[test]
    fn fs_enum_non() {
        use crate::{FlatSerializable, WrapErr};
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&1u16.to_ne_bytes());

        let res= unsafe { Bar::try_ref(&bytes) };
        assert!(matches!(res, Err(WrapErr::InvalidTag(0))));
    }
}
