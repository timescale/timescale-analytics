#![allow(unused_imports)]
use crate as flat_serialize;
#[derive(Clone, Debug)]
pub struct Basic<'input> {
    pub header: u64,
    pub data_len: u32,
    pub array: [u16; 3],
    pub data: <u8 as flat_serialize::FlatSerializable<'input>>::SLICE,
    pub data2: <[u8; 2] as flat_serialize::FlatSerializable<'input>>::SLICE,
}
#[allow(unused_assignments)]
const _: () = {
    use std::mem::{align_of, size_of};
    let mut current_size = 0;
    let mut min_align = 8;
    let _alignment_check: () =
        [()][(current_size) % <u64 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT];
    let _alignment_check2: () = [()][(<u64 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT
        > min_align) as u8 as usize];
    current_size += <u64 as flat_serialize::FlatSerializable>::MIN_LEN;
    min_align = match <u64 as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT {
        Some(align) if align < min_align => align,
        _ => min_align,
    };
    let _alignment_check: () =
        [()][(current_size) % <u32 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT];
    let _alignment_check2: () = [()][(<u32 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT
        > min_align) as u8 as usize];
    current_size += <u32 as flat_serialize::FlatSerializable>::MIN_LEN;
    min_align = match <u32 as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT {
        Some(align) if align < min_align => align,
        _ => min_align,
    };
    let _alignment_check: () =
        [()][(current_size) % <[u16; 3] as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT];
    let _alignment_check2: () = [()]
        [(<[u16; 3] as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT > min_align) as u8
            as usize];
    current_size += <[u16; 3] as flat_serialize::FlatSerializable>::MIN_LEN;
    min_align = match <[u16; 3] as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT {
        Some(align) if align < min_align => align,
        _ => min_align,
    };
    let _alignment_check: () =
        [()][(current_size) % <u8 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT];
    let _alignment_check2: () = [()]
        [(<u8 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT > min_align) as u8 as usize];
    if <u8 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT < min_align {
        min_align = <u8 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT
    }
    min_align = match <u8 as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT {
        Some(align) if align < min_align => align,
        _ => min_align,
    };
    let _alignment_check: () =
        [()][(current_size) % <[u8; 2] as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT];
    let _alignment_check2: () = [()]
        [(<[u8; 2] as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT > min_align) as u8
            as usize];
    if <[u8; 2] as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT < min_align {
        min_align = <[u8; 2] as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT
    }
    min_align = match <[u8; 2] as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT {
        Some(align) if align < min_align => align,
        _ => min_align,
    };
};
const _: () = {
    fn header<'test, T: flat_serialize::FlatSerializable<'test>>() {}
    let _ = header::<u64>;
    fn data_len<'test, T: flat_serialize::FlatSerializable<'test>>() {}
    let _ = data_len::<u32>;
    fn array<'test, T: flat_serialize::FlatSerializable<'test>>() {}
    let _ = array::<[u16; 3]>;
    fn data<'test, T: flat_serialize::FlatSerializable<'test>>() {}
    let _ = data::<u8>;
    fn data2<'test, T: flat_serialize::FlatSerializable<'test>>() {}
    let _ = data2::<[u8; 2]>;
};
unsafe impl<'input> flat_serialize::FlatSerializable<'input> for Basic<'input> {
    const REQUIRED_ALIGNMENT: usize = {
        use std::mem::align_of;
        let mut required_alignment = 1;
        let alignment = <u64 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT;
        if alignment > required_alignment {
            required_alignment = alignment;
        }
        let alignment = <u32 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT;
        if alignment > required_alignment {
            required_alignment = alignment;
        }
        let alignment = <[u16; 3] as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT;
        if alignment > required_alignment {
            required_alignment = alignment;
        }
        let alignment = <u8 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT;
        if alignment > required_alignment {
            required_alignment = alignment;
        }
        let alignment = <[u8; 2] as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT;
        if alignment > required_alignment {
            required_alignment = alignment;
        }
        required_alignment
    };
    const MAX_PROVIDED_ALIGNMENT: Option<usize> = {
        use std::mem::align_of;
        let mut min_align: Option<usize> = None;
        let ty_align = <u64 as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT;
        match (ty_align, min_align) {
            (None, _) => (),
            (Some(align), None) => min_align = Some(align),
            (Some(align), Some(min)) if align < min => min_align = Some(align),
            _ => (),
        }
        let ty_align = <u32 as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT;
        match (ty_align, min_align) {
            (None, _) => (),
            (Some(align), None) => min_align = Some(align),
            (Some(align), Some(min)) if align < min => min_align = Some(align),
            _ => (),
        }
        let ty_align = <[u16; 3] as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT;
        match (ty_align, min_align) {
            (None, _) => (),
            (Some(align), None) => min_align = Some(align),
            (Some(align), Some(min)) if align < min => min_align = Some(align),
            _ => (),
        }
        let ty_align = { Some(<u8 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT) };
        match (ty_align, min_align) {
            (None, _) => (),
            (Some(align), None) => min_align = Some(align),
            (Some(align), Some(min)) if align < min => min_align = Some(align),
            _ => (),
        }
        let ty_align = { Some(<[u8; 2] as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT) };
        match (ty_align, min_align) {
            (None, _) => (),
            (Some(align), None) => min_align = Some(align),
            (Some(align), Some(min)) if align < min => min_align = Some(align),
            _ => (),
        }
        match min_align {
            None => None,
            Some(min_align) => {
                let min_size = Self::MIN_LEN;
                if min_size % 8 == 0 && min_align >= 8 {
                    Some(8)
                } else if min_size % 4 == 0 && min_align >= 4 {
                    Some(4)
                } else if min_size % 2 == 0 && min_align >= 2 {
                    Some(2)
                } else {
                    Some(1)
                }
            }
        }
    };
    const MIN_LEN: usize = {
        use std::mem::size_of;
        let mut size = 0;
        size += <u64 as flat_serialize::FlatSerializable>::MIN_LEN;
        size += <u32 as flat_serialize::FlatSerializable>::MIN_LEN;
        size += <[u16; 3] as flat_serialize::FlatSerializable>::MIN_LEN;
        size += 0;
        size += 0;
        size
    };
    const TRIVIAL_COPY: bool = false;
    type SLICE = flat_serialize::Iterable<'input, Basic<'input>>;
    #[allow(unused_assignments, unused_variables)]
    #[inline(always)]
    unsafe fn try_ref(
        mut input: &'input [u8],
    ) -> Result<(Self, &'input [u8]), flat_serialize::WrapErr> {
        if input.len() < Self::MIN_LEN {
            return Err(flat_serialize::WrapErr::NotEnoughBytes(Self::MIN_LEN));
        }
        let __packet_macro_read_len = 0usize;
        let mut header: Option<u64> = None;
        let mut data_len: Option<u32> = None;
        let mut array: Option<[u16; 3]> = None;
        let mut data: Option<<u8 as flat_serialize::FlatSerializable<'_>>::SLICE> = None;
        let mut data2: Option<<[u8; 2] as flat_serialize::FlatSerializable<'_>>::SLICE> = None;
        'tryref: loop {
            {
                let (field, rem) = match <u64>::try_ref(input) {
                    Ok((f, b)) => (f, b),
                    Err(flat_serialize::WrapErr::InvalidTag(offset)) => {
                        return Err(flat_serialize::WrapErr::InvalidTag(
                            __packet_macro_read_len + offset,
                        ))
                    }
                    Err(..) => break 'tryref,
                };
                input = rem;
                header = Some(field);
            }
            {
                let (field, rem) = match <u32>::try_ref(input) {
                    Ok((f, b)) => (f, b),
                    Err(flat_serialize::WrapErr::InvalidTag(offset)) => {
                        return Err(flat_serialize::WrapErr::InvalidTag(
                            __packet_macro_read_len + offset,
                        ))
                    }
                    Err(..) => break 'tryref,
                };
                input = rem;
                data_len = Some(field);
            }
            {
                let (field, rem) = match <[u16; 3]>::try_ref(input) {
                    Ok((f, b)) => (f, b),
                    Err(flat_serialize::WrapErr::InvalidTag(offset)) => {
                        return Err(flat_serialize::WrapErr::InvalidTag(
                            __packet_macro_read_len + offset,
                        ))
                    }
                    Err(..) => break 'tryref,
                };
                input = rem;
                array = Some(field);
            }
            {
                let count = (data_len.clone().unwrap()) as usize;
                let (field, rem) = match <_ as flat_serialize::Slice<'_>>::try_ref(input, count) {
                    Ok((f, b)) => (f, b),
                    Err(flat_serialize::WrapErr::InvalidTag(offset)) => {
                        return Err(flat_serialize::WrapErr::InvalidTag(
                            __packet_macro_read_len + offset,
                        ))
                    }
                    Err(..) => break 'tryref,
                };
                input = rem;
                data = Some(field);
            }
            {
                let count = (data_len.clone().unwrap() / 3) as usize;
                let (field, rem) = match <_ as flat_serialize::Slice<'_>>::try_ref(input, count) {
                    Ok((f, b)) => (f, b),
                    Err(flat_serialize::WrapErr::InvalidTag(offset)) => {
                        return Err(flat_serialize::WrapErr::InvalidTag(
                            __packet_macro_read_len + offset,
                        ))
                    }
                    Err(..) => break 'tryref,
                };
                input = rem;
                data2 = Some(field);
            }
            let _ref = Basic {
                header: header.unwrap(),
                data_len: data_len.unwrap(),
                array: array.unwrap(),
                data: data.unwrap(),
                data2: data2.unwrap(),
            };
            return Ok((_ref, input));
        }
        Err(flat_serialize::WrapErr::NotEnoughBytes(
            0 + <u64>::MIN_LEN
                + <u32>::MIN_LEN
                + <[u16; 3]>::MIN_LEN
                + (|| {
                    <u8>::MIN_LEN
                        * (match data_len {
                            Some(data_len) => data_len,
                            None => return 0usize,
                        }) as usize
                })()
                + (|| {
                    <[u8; 2]>::MIN_LEN
                        * (match data_len {
                            Some(data_len) => data_len,
                            None => return 0usize,
                        } / 3) as usize
                })(),
        ))
    }
    #[allow(unused_assignments, unused_variables)]
    #[inline(always)]
    unsafe fn fill_slice<'out>(
        &self,
        input: &'out mut [std::mem::MaybeUninit<u8>],
    ) -> &'out mut [std::mem::MaybeUninit<u8>] {
        let total_len = self.len();
        let (mut input, rem) = input.split_at_mut(total_len);
        let Basic {
            header,
            data_len,
            array,
            data,
            data2,
        } = self;
        unsafe {
            input = header.fill_slice(input);
        };
        unsafe {
            input = data_len.fill_slice(input);
        };
        unsafe {
            input = array.fill_slice(input);
        };
        unsafe {
            let count = (*data_len) as usize;
            input = <_ as flat_serialize::Slice<'_>>::fill_slice(data, count, input);
        };
        unsafe {
            let count = ((*data_len) / 3) as usize;
            input = <_ as flat_serialize::Slice<'_>>::fill_slice(data2, count, input);
        }
        debug_assert_eq!(input.len(), 0);
        rem
    }
    #[allow(unused_assignments, unused_variables)]
    #[inline(always)]
    fn len(&self) -> usize {
        let Basic {
            header,
            data_len,
            array,
            data,
            data2,
        } = self;
        0usize
            + <u64 as flat_serialize::FlatSerializable>::len(header)
            + <u32 as flat_serialize::FlatSerializable>::len(data_len)
            + <[u16; 3] as flat_serialize::FlatSerializable>::len(array)
            + (<_ as flat_serialize::Slice<'_>>::len(data, (*data_len) as usize))
            + (<_ as flat_serialize::Slice<'_>>::len(data2, ((*data_len) / 3) as usize))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Optional {
    pub header: u64,
    pub optional_field: Option<u32>,
    pub non_optional_field: u16,
}
#[allow(unused_assignments)]
const _: () = {
    use std::mem::{align_of, size_of};
    let mut current_size = 0;
    let mut min_align = 8;
    let _alignment_check: () =
        [()][(current_size) % <u64 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT];
    let _alignment_check2: () = [()][(<u64 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT
        > min_align) as u8 as usize];
    current_size += <u64 as flat_serialize::FlatSerializable>::MIN_LEN;
    min_align = match <u64 as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT {
        Some(align) if align < min_align => align,
        _ => min_align,
    };
    let _alignment_check: () =
        [()][(current_size) % <u32 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT];
    let _alignment_check2: () = [()][(<u32 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT
        > min_align) as u8 as usize];
    if <u32 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT < min_align {
        min_align = <u32 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT
    }
    min_align = match <u32 as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT {
        Some(align) if align < min_align => align,
        _ => min_align,
    };
    let _alignment_check: () =
        [()][(current_size) % <u16 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT];
    let _alignment_check2: () = [()][(<u16 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT
        > min_align) as u8 as usize];
    current_size += <u16 as flat_serialize::FlatSerializable>::MIN_LEN;
    min_align = match <u16 as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT {
        Some(align) if align < min_align => align,
        _ => min_align,
    };
};
const _: () = {
    fn header<'test, T: flat_serialize::FlatSerializable<'test>>() {}
    let _ = header::<u64>;
    fn optional_field<'test, T: flat_serialize::FlatSerializable<'test>>() {}
    let _ = optional_field::<u32>;
    fn non_optional_field<'test, T: flat_serialize::FlatSerializable<'test>>() {}
    let _ = non_optional_field::<u16>;
};
unsafe impl<'a> flat_serialize::FlatSerializable<'a> for Optional {
    const REQUIRED_ALIGNMENT: usize = {
        use std::mem::align_of;
        let mut required_alignment = 1;
        let alignment = <u64 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT;
        if alignment > required_alignment {
            required_alignment = alignment;
        }
        let alignment = <u32 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT;
        if alignment > required_alignment {
            required_alignment = alignment;
        }
        let alignment = <u16 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT;
        if alignment > required_alignment {
            required_alignment = alignment;
        }
        required_alignment
    };
    const MAX_PROVIDED_ALIGNMENT: Option<usize> = {
        use std::mem::align_of;
        let mut min_align: Option<usize> = None;
        let ty_align = <u64 as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT;
        match (ty_align, min_align) {
            (None, _) => (),
            (Some(align), None) => min_align = Some(align),
            (Some(align), Some(min)) if align < min => min_align = Some(align),
            _ => (),
        }
        let ty_align = {
            let ty_provied = <u32 as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT;
            match ty_provied {
                Some(align) => Some(align),
                None => Some(<u32 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT),
            }
        };
        match (ty_align, min_align) {
            (None, _) => (),
            (Some(align), None) => min_align = Some(align),
            (Some(align), Some(min)) if align < min => min_align = Some(align),
            _ => (),
        }
        let ty_align = <u16 as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT;
        match (ty_align, min_align) {
            (None, _) => (),
            (Some(align), None) => min_align = Some(align),
            (Some(align), Some(min)) if align < min => min_align = Some(align),
            _ => (),
        }
        match min_align {
            None => None,
            Some(min_align) => {
                let min_size = Self::MIN_LEN;
                if min_size % 8 == 0 && min_align >= 8 {
                    Some(8)
                } else if min_size % 4 == 0 && min_align >= 4 {
                    Some(4)
                } else if min_size % 2 == 0 && min_align >= 2 {
                    Some(2)
                } else {
                    Some(1)
                }
            }
        }
    };
    const MIN_LEN: usize = {
        use std::mem::size_of;
        let mut size = 0;
        size += <u64 as flat_serialize::FlatSerializable>::MIN_LEN;
        size += 0;
        size += <u16 as flat_serialize::FlatSerializable>::MIN_LEN;
        size
    };
    const TRIVIAL_COPY: bool = false;
    type SLICE = flat_serialize::Iterable<'a, Optional>;
    #[allow(unused_assignments, unused_variables)]
    #[inline(always)]
    unsafe fn try_ref(mut input: &[u8]) -> Result<(Self, &[u8]), flat_serialize::WrapErr> {
        if input.len() < Self::MIN_LEN {
            return Err(flat_serialize::WrapErr::NotEnoughBytes(Self::MIN_LEN));
        }
        let __packet_macro_read_len = 0usize;
        let mut header: Option<u64> = None;
        let mut optional_field: Option<u32> = None;
        let mut non_optional_field: Option<u16> = None;
        'tryref: loop {
            {
                let (field, rem) = match <u64>::try_ref(input) {
                    Ok((f, b)) => (f, b),
                    Err(flat_serialize::WrapErr::InvalidTag(offset)) => {
                        return Err(flat_serialize::WrapErr::InvalidTag(
                            __packet_macro_read_len + offset,
                        ))
                    }
                    Err(..) => break 'tryref,
                };
                input = rem;
                header = Some(field);
            }
            if header.clone().unwrap() != 1 {
                let (field, rem) = match <u32>::try_ref(input) {
                    Ok((f, b)) => (f, b),
                    Err(flat_serialize::WrapErr::InvalidTag(offset)) => {
                        return Err(flat_serialize::WrapErr::InvalidTag(
                            __packet_macro_read_len + offset,
                        ))
                    }
                    Err(..) => break 'tryref,
                };
                input = rem;
                optional_field = Some(field);
            }
            {
                let (field, rem) = match <u16>::try_ref(input) {
                    Ok((f, b)) => (f, b),
                    Err(flat_serialize::WrapErr::InvalidTag(offset)) => {
                        return Err(flat_serialize::WrapErr::InvalidTag(
                            __packet_macro_read_len + offset,
                        ))
                    }
                    Err(..) => break 'tryref,
                };
                input = rem;
                non_optional_field = Some(field);
            }
            let _ref = Optional {
                header: header.unwrap(),
                optional_field: optional_field,
                non_optional_field: non_optional_field.unwrap(),
            };
            return Ok((_ref, input));
        }
        Err(flat_serialize::WrapErr::NotEnoughBytes(
            0 + <u64>::MIN_LEN
                + (|| {
                    if match header {
                        Some(header) => header,
                        None => return 0usize,
                    } != 1
                    {
                        <u32>::MIN_LEN
                    } else {
                        0
                    }
                })()
                + <u16>::MIN_LEN,
        ))
    }
    #[allow(unused_assignments, unused_variables)]
    #[inline(always)]
    unsafe fn fill_slice<'out>(
        &self,
        input: &'out mut [std::mem::MaybeUninit<u8>],
    ) -> &'out mut [std::mem::MaybeUninit<u8>] {
        let total_len = self.len();
        let (mut input, rem) = input.split_at_mut(total_len);
        let Optional {
            header,
            optional_field,
            non_optional_field,
        } = self;
        unsafe {
            input = header.fill_slice(input);
        };
        unsafe {
            if (*header) != 1 {
                let optional_field: &u32 = optional_field.as_ref().unwrap();
                input = optional_field.fill_slice(input);
            }
        };
        unsafe {
            input = non_optional_field.fill_slice(input);
        }
        debug_assert_eq!(input.len(), 0);
        rem
    }
    #[allow(unused_assignments, unused_variables)]
    #[inline(always)]
    fn len(&self) -> usize {
        let Optional {
            header,
            optional_field,
            non_optional_field,
        } = self;
        0usize
            + <u64 as flat_serialize::FlatSerializable>::len(header)
            + (if (*header) != 1 {
                <u32 as flat_serialize::FlatSerializable>::len(optional_field.as_ref().unwrap())
            } else {
                0
            })
            + <u16 as flat_serialize::FlatSerializable>::len(non_optional_field)
    }
}
#[derive(Clone, Debug)]
pub struct Nested<'a> {
    pub prefix: u64,
    pub basic: Basic<'a>,
}
#[allow(unused_assignments)]
const _: () = {
    use std::mem::{align_of, size_of};
    let mut current_size = 0;
    let mut min_align = 8;
    let _alignment_check: () =
        [()][(current_size) % <u64 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT];
    let _alignment_check2: () = [()][(<u64 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT
        > min_align) as u8 as usize];
    current_size += <u64 as flat_serialize::FlatSerializable>::MIN_LEN;
    min_align = match <u64 as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT {
        Some(align) if align < min_align => align,
        _ => min_align,
    };
    let _alignment_check: () =
        [()][(current_size) % <Basic as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT];
    let _alignment_check2: () = [()]
        [(<Basic as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT > min_align) as u8
            as usize];
    current_size += <Basic as flat_serialize::FlatSerializable>::MIN_LEN;
    min_align = match <Basic as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT {
        Some(align) if align < min_align => align,
        _ => min_align,
    };
};
const _: () = {
    fn prefix<'test, T: flat_serialize::FlatSerializable<'test>>() {}
    let _ = prefix::<u64>;
    fn basic<'test, T: flat_serialize::FlatSerializable<'test>>() {}
    let _ = basic::<Basic<'static>>;
};
unsafe impl<'a> flat_serialize::FlatSerializable<'a> for Nested<'a> {
    const REQUIRED_ALIGNMENT: usize = {
        use std::mem::align_of;
        let mut required_alignment = 1;
        let alignment = <u64 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT;
        if alignment > required_alignment {
            required_alignment = alignment;
        }
        let alignment = <Basic as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT;
        if alignment > required_alignment {
            required_alignment = alignment;
        }
        required_alignment
    };
    const MAX_PROVIDED_ALIGNMENT: Option<usize> = {
        use std::mem::align_of;
        let mut min_align: Option<usize> = None;
        let ty_align = <u64 as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT;
        match (ty_align, min_align) {
            (None, _) => (),
            (Some(align), None) => min_align = Some(align),
            (Some(align), Some(min)) if align < min => min_align = Some(align),
            _ => (),
        }
        let ty_align = <Basic as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT;
        match (ty_align, min_align) {
            (None, _) => (),
            (Some(align), None) => min_align = Some(align),
            (Some(align), Some(min)) if align < min => min_align = Some(align),
            _ => (),
        }
        match min_align {
            None => None,
            Some(min_align) => {
                let min_size = Self::MIN_LEN;
                if min_size % 8 == 0 && min_align >= 8 {
                    Some(8)
                } else if min_size % 4 == 0 && min_align >= 4 {
                    Some(4)
                } else if min_size % 2 == 0 && min_align >= 2 {
                    Some(2)
                } else {
                    Some(1)
                }
            }
        }
    };
    const MIN_LEN: usize = {
        use std::mem::size_of;
        let mut size = 0;
        size += <u64 as flat_serialize::FlatSerializable>::MIN_LEN;
        size += <Basic as flat_serialize::FlatSerializable>::MIN_LEN;
        size
    };
    const TRIVIAL_COPY: bool = false;
    type SLICE = flat_serialize::Iterable<'a, Nested<'a>>;
    #[allow(unused_assignments, unused_variables)]
    #[inline(always)]
    unsafe fn try_ref(mut input: &'a [u8]) -> Result<(Self, &'a [u8]), flat_serialize::WrapErr> {
        if input.len() < Self::MIN_LEN {
            return Err(flat_serialize::WrapErr::NotEnoughBytes(Self::MIN_LEN));
        }
        let __packet_macro_read_len = 0usize;
        let mut prefix: Option<u64> = None;
        let mut basic: Option<Basic<'a>> = None;
        'tryref: loop {
            {
                let (field, rem) = match <u64>::try_ref(input) {
                    Ok((f, b)) => (f, b),
                    Err(flat_serialize::WrapErr::InvalidTag(offset)) => {
                        return Err(flat_serialize::WrapErr::InvalidTag(
                            __packet_macro_read_len + offset,
                        ))
                    }
                    Err(..) => break 'tryref,
                };
                input = rem;
                prefix = Some(field);
            }
            {
                let (field, rem) = match <Basic>::try_ref(input) {
                    Ok((f, b)) => (f, b),
                    Err(flat_serialize::WrapErr::InvalidTag(offset)) => {
                        return Err(flat_serialize::WrapErr::InvalidTag(
                            __packet_macro_read_len + offset,
                        ))
                    }
                    Err(..) => break 'tryref,
                };
                input = rem;
                basic = Some(field);
            }
            let _ref = Nested {
                prefix: prefix.unwrap(),
                basic: basic.unwrap(),
            };
            return Ok((_ref, input));
        }
        Err(flat_serialize::WrapErr::NotEnoughBytes(
            0 + <u64>::MIN_LEN + <Basic>::MIN_LEN,
        ))
    }
    #[allow(unused_assignments, unused_variables)]
    #[inline(always)]
    unsafe fn fill_slice<'out>(
        &self,
        input: &'out mut [std::mem::MaybeUninit<u8>],
    ) -> &'out mut [std::mem::MaybeUninit<u8>] {
        let total_len = self.len();
        let (mut input, rem) = input.split_at_mut(total_len);
        let Nested { prefix, basic } = self;
        unsafe {
            input = prefix.fill_slice(input);
        };
        unsafe {
            input = basic.fill_slice(input);
        }
        debug_assert_eq!(input.len(), 0);
        rem
    }
    #[allow(unused_assignments, unused_variables)]
    #[inline(always)]
    fn len(&self) -> usize {
        let Nested { prefix, basic } = self;
        0usize
            + <u64 as flat_serialize::FlatSerializable>::len(prefix)
            + <Basic as flat_serialize::FlatSerializable>::len(basic)
    }
}
#[derive(Clone, Debug)]
pub struct NestedOptional {
    pub present: u64,
    pub val: Option<Optional>,
}
#[allow(unused_assignments)]
const _: () = {
    use std::mem::{align_of, size_of};
    let mut current_size = 0;
    let mut min_align = 8;
    let _alignment_check: () =
        [()][(current_size) % <u64 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT];
    let _alignment_check2: () = [()][(<u64 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT
        > min_align) as u8 as usize];
    current_size += <u64 as flat_serialize::FlatSerializable>::MIN_LEN;
    min_align = match <u64 as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT {
        Some(align) if align < min_align => align,
        _ => min_align,
    };
    let _alignment_check: () =
        [()][(current_size) % <Optional as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT];
    let _alignment_check2: () = [()]
        [(<Optional as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT > min_align) as u8
            as usize];
    if <Optional as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT < min_align {
        min_align = <Optional as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT
    }
    min_align = match <Optional as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT {
        Some(align) if align < min_align => align,
        _ => min_align,
    };
};
const _: () = {
    fn present<'test, T: flat_serialize::FlatSerializable<'test>>() {}
    let _ = present::<u64>;
    fn val<'test, T: flat_serialize::FlatSerializable<'test>>() {}
    let _ = val::<Optional>;
};
unsafe impl<'a> flat_serialize::FlatSerializable<'a> for NestedOptional {
    const REQUIRED_ALIGNMENT: usize = {
        use std::mem::align_of;
        let mut required_alignment = 1;
        let alignment = <u64 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT;
        if alignment > required_alignment {
            required_alignment = alignment;
        }
        let alignment = <Optional as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT;
        if alignment > required_alignment {
            required_alignment = alignment;
        }
        required_alignment
    };
    const MAX_PROVIDED_ALIGNMENT: Option<usize> = {
        use std::mem::align_of;
        let mut min_align: Option<usize> = None;
        let ty_align = <u64 as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT;
        match (ty_align, min_align) {
            (None, _) => (),
            (Some(align), None) => min_align = Some(align),
            (Some(align), Some(min)) if align < min => min_align = Some(align),
            _ => (),
        }
        let ty_align = {
            let ty_provied = <Optional as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT;
            match ty_provied {
                Some(align) => Some(align),
                None => Some(<Optional as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT),
            }
        };
        match (ty_align, min_align) {
            (None, _) => (),
            (Some(align), None) => min_align = Some(align),
            (Some(align), Some(min)) if align < min => min_align = Some(align),
            _ => (),
        }
        match min_align {
            None => None,
            Some(min_align) => {
                let min_size = Self::MIN_LEN;
                if min_size % 8 == 0 && min_align >= 8 {
                    Some(8)
                } else if min_size % 4 == 0 && min_align >= 4 {
                    Some(4)
                } else if min_size % 2 == 0 && min_align >= 2 {
                    Some(2)
                } else {
                    Some(1)
                }
            }
        }
    };
    const MIN_LEN: usize = {
        use std::mem::size_of;
        let mut size = 0;
        size += <u64 as flat_serialize::FlatSerializable>::MIN_LEN;
        size += 0;
        size
    };
    const TRIVIAL_COPY: bool = false;
    type SLICE = flat_serialize::Iterable<'a, NestedOptional>;
    #[allow(unused_assignments, unused_variables)]
    #[inline(always)]
    unsafe fn try_ref(mut input: &[u8]) -> Result<(Self, &[u8]), flat_serialize::WrapErr> {
        if input.len() < Self::MIN_LEN {
            return Err(flat_serialize::WrapErr::NotEnoughBytes(Self::MIN_LEN));
        }
        let __packet_macro_read_len = 0usize;
        let mut present: Option<u64> = None;
        let mut val: Option<Optional> = None;
        'tryref: loop {
            {
                let (field, rem) = match <u64>::try_ref(input) {
                    Ok((f, b)) => (f, b),
                    Err(flat_serialize::WrapErr::InvalidTag(offset)) => {
                        return Err(flat_serialize::WrapErr::InvalidTag(
                            __packet_macro_read_len + offset,
                        ))
                    }
                    Err(..) => break 'tryref,
                };
                input = rem;
                present = Some(field);
            }
            if present.clone().unwrap() > 2 {
                let (field, rem) = match <Optional>::try_ref(input) {
                    Ok((f, b)) => (f, b),
                    Err(flat_serialize::WrapErr::InvalidTag(offset)) => {
                        return Err(flat_serialize::WrapErr::InvalidTag(
                            __packet_macro_read_len + offset,
                        ))
                    }
                    Err(..) => break 'tryref,
                };
                input = rem;
                val = Some(field);
            }
            let _ref = NestedOptional {
                present: present.unwrap(),
                val: val,
            };
            return Ok((_ref, input));
        }
        Err(flat_serialize::WrapErr::NotEnoughBytes(
            0 + <u64>::MIN_LEN
                + (|| {
                    if match present {
                        Some(present) => present,
                        None => return 0usize,
                    } > 2
                    {
                        <Optional>::MIN_LEN
                    } else {
                        0
                    }
                })(),
        ))
    }
    #[allow(unused_assignments, unused_variables)]
    #[inline(always)]
    unsafe fn fill_slice<'out>(
        &self,
        input: &'out mut [std::mem::MaybeUninit<u8>],
    ) -> &'out mut [std::mem::MaybeUninit<u8>] {
        let total_len = self.len();
        let (mut input, rem) = input.split_at_mut(total_len);
        let NestedOptional { present, val } = self;
        unsafe {
            input = present.fill_slice(input);
        };
        unsafe {
            if (*present) > 2 {
                let val: &Optional = val.as_ref().unwrap();
                input = val.fill_slice(input);
            }
        }
        debug_assert_eq!(input.len(), 0);
        rem
    }
    #[allow(unused_assignments, unused_variables)]
    #[inline(always)]
    fn len(&self) -> usize {
        let NestedOptional { present, val } = self;
        0usize
            + <u64 as flat_serialize::FlatSerializable>::len(present)
            + (if (*present) > 2 {
                <Optional as flat_serialize::FlatSerializable>::len(val.as_ref().unwrap())
            } else {
                0
            })
    }
}
#[derive(Clone, Debug)]
pub struct NestedSlice<'b> {
    pub num_vals: u64,
    pub vals: <Optional as flat_serialize::FlatSerializable<'b>>::SLICE,
}
#[allow(unused_assignments)]
const _: () = {
    use std::mem::{align_of, size_of};
    let mut current_size = 0;
    let mut min_align = 8;
    let _alignment_check: () =
        [()][(current_size) % <u64 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT];
    let _alignment_check2: () = [()][(<u64 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT
        > min_align) as u8 as usize];
    current_size += <u64 as flat_serialize::FlatSerializable>::MIN_LEN;
    min_align = match <u64 as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT {
        Some(align) if align < min_align => align,
        _ => min_align,
    };
    let _alignment_check: () =
        [()][(current_size) % <Optional as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT];
    let _alignment_check2: () = [()]
        [(<Optional as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT > min_align) as u8
            as usize];
    if <Optional as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT < min_align {
        min_align = <Optional as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT
    }
    min_align = match <Optional as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT {
        Some(align) if align < min_align => align,
        _ => min_align,
    };
};
const _: () = {
    fn num_vals<'test, T: flat_serialize::FlatSerializable<'test>>() {}
    let _ = num_vals::<u64>;
    fn vals<'test, T: flat_serialize::FlatSerializable<'test>>() {}
    let _ = vals::<Optional>;
};
unsafe impl<'b> flat_serialize::FlatSerializable<'b> for NestedSlice<'b> {
    const REQUIRED_ALIGNMENT: usize = {
        use std::mem::align_of;
        let mut required_alignment = 1;
        let alignment = <u64 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT;
        if alignment > required_alignment {
            required_alignment = alignment;
        }
        let alignment = <Optional as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT;
        if alignment > required_alignment {
            required_alignment = alignment;
        }
        required_alignment
    };
    const MAX_PROVIDED_ALIGNMENT: Option<usize> = {
        use std::mem::align_of;
        let mut min_align: Option<usize> = None;
        let ty_align = <u64 as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT;
        match (ty_align, min_align) {
            (None, _) => (),
            (Some(align), None) => min_align = Some(align),
            (Some(align), Some(min)) if align < min => min_align = Some(align),
            _ => (),
        }
        let ty_align = { Some(<Optional as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT) };
        match (ty_align, min_align) {
            (None, _) => (),
            (Some(align), None) => min_align = Some(align),
            (Some(align), Some(min)) if align < min => min_align = Some(align),
            _ => (),
        }
        match min_align {
            None => None,
            Some(min_align) => {
                let min_size = Self::MIN_LEN;
                if min_size % 8 == 0 && min_align >= 8 {
                    Some(8)
                } else if min_size % 4 == 0 && min_align >= 4 {
                    Some(4)
                } else if min_size % 2 == 0 && min_align >= 2 {
                    Some(2)
                } else {
                    Some(1)
                }
            }
        }
    };
    const MIN_LEN: usize = {
        use std::mem::size_of;
        let mut size = 0;
        size += <u64 as flat_serialize::FlatSerializable>::MIN_LEN;
        size += 0;
        size
    };
    const TRIVIAL_COPY: bool = false;
    type SLICE = flat_serialize::Iterable<'b, NestedSlice<'b>>;
    #[allow(unused_assignments, unused_variables)]
    #[inline(always)]
    unsafe fn try_ref(mut input: &'b [u8]) -> Result<(Self, &'b [u8]), flat_serialize::WrapErr> {
        if input.len() < Self::MIN_LEN {
            return Err(flat_serialize::WrapErr::NotEnoughBytes(Self::MIN_LEN));
        }
        let __packet_macro_read_len = 0usize;
        let mut num_vals: Option<u64> = None;
        let mut vals: Option<<Optional as flat_serialize::FlatSerializable<'_>>::SLICE> = None;
        'tryref: loop {
            {
                let (field, rem) = match <u64>::try_ref(input) {
                    Ok((f, b)) => (f, b),
                    Err(flat_serialize::WrapErr::InvalidTag(offset)) => {
                        return Err(flat_serialize::WrapErr::InvalidTag(
                            __packet_macro_read_len + offset,
                        ))
                    }
                    Err(..) => break 'tryref,
                };
                input = rem;
                num_vals = Some(field);
            }
            {
                let count = (num_vals.clone().unwrap()) as usize;
                let (field, rem) = match <_ as flat_serialize::Slice<'_>>::try_ref(input, count) {
                    Ok((f, b)) => (f, b),
                    Err(flat_serialize::WrapErr::InvalidTag(offset)) => {
                        return Err(flat_serialize::WrapErr::InvalidTag(
                            __packet_macro_read_len + offset,
                        ))
                    }
                    Err(..) => break 'tryref,
                };
                input = rem;
                vals = Some(field);
            }
            let _ref = NestedSlice {
                num_vals: num_vals.unwrap(),
                vals: vals.unwrap(),
            };
            return Ok((_ref, input));
        }
        Err(flat_serialize::WrapErr::NotEnoughBytes(
            0 + <u64>::MIN_LEN
                + (|| {
                    <Optional>::MIN_LEN
                        * (match num_vals {
                            Some(num_vals) => num_vals,
                            None => return 0usize,
                        }) as usize
                })(),
        ))
    }
    #[allow(unused_assignments, unused_variables)]
    #[inline(always)]
    unsafe fn fill_slice<'out>(
        &self,
        input: &'out mut [std::mem::MaybeUninit<u8>],
    ) -> &'out mut [std::mem::MaybeUninit<u8>] {
        let total_len = self.len();
        let (mut input, rem) = input.split_at_mut(total_len);
        let NestedSlice { num_vals, vals } = self;
        unsafe {
            input = num_vals.fill_slice(input);
        };
        unsafe {
            let count = (*num_vals) as usize;
            input = <_ as flat_serialize::Slice<'_>>::fill_slice(vals, count, input);
        }
        debug_assert_eq!(input.len(), 0);
        rem
    }
    #[allow(unused_assignments, unused_variables)]
    #[inline(always)]
    fn len(&self) -> usize {
        let NestedSlice { num_vals, vals } = self;
        0usize
            + <u64 as flat_serialize::FlatSerializable>::len(num_vals)
            + (<_ as flat_serialize::Slice<'_>>::len(vals, (*num_vals) as usize))
    }
}
#[derive(Clone, Debug)]
pub enum BasicEnum<'input> {
    First {
        data_len: u32,
        data: <u8 as flat_serialize::FlatSerializable<'input>>::SLICE,
    },
    Fixed {
        array: [u16; 3],
    },
}
#[allow(unused_assignments)]
const _: () = {
    use std::mem::{align_of, size_of};
    let mut current_size = 0;
    let mut min_align = 8;
    let _alignment_check: () =
        [()][(current_size) % <u64 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT];
    let _alignment_check2: () = [()][(<u64 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT
        > min_align) as u8 as usize];
    current_size += <u64 as flat_serialize::FlatSerializable>::MIN_LEN;
    min_align = match <u64 as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT {
        Some(align) if align < min_align => align,
        _ => min_align,
    };
    {
        use std::mem::{align_of, size_of};
        let mut current_size = current_size;
        let mut min_align = min_align;
        let _alignment_check: () =
            [()][(current_size) % <u32 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT];
        let _alignment_check2: () = [()]
            [(<u32 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT > min_align) as u8
                as usize];
        current_size += <u32 as flat_serialize::FlatSerializable>::MIN_LEN;
        min_align = match <u32 as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT {
            Some(align) if align < min_align => align,
            _ => min_align,
        };
        let _alignment_check: () =
            [()][(current_size) % <u8 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT];
        let _alignment_check2: () = [()]
            [(<u8 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT > min_align) as u8
                as usize];
        if <u8 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT < min_align {
            min_align = <u8 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT
        }
        min_align = match <u8 as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT {
            Some(align) if align < min_align => align,
            _ => min_align,
        };
    }
    {
        use std::mem::{align_of, size_of};
        let mut current_size = current_size;
        let mut min_align = min_align;
        let _alignment_check: () = [()]
            [(current_size) % <[u16; 3] as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT];
        let _alignment_check2: () = [()]
            [(<[u16; 3] as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT > min_align) as u8
                as usize];
        current_size += <[u16; 3] as flat_serialize::FlatSerializable>::MIN_LEN;
        min_align = match <[u16; 3] as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT {
            Some(align) if align < min_align => align,
            _ => min_align,
        };
    }
};
const _: () = {
    #[allow(dead_code)]
    enum UniquenessCheck {
        First = 2,
        Fixed = 3,
    }
};
const _: () = {
    fn k<'test, T: flat_serialize::FlatSerializable<'test>>() {}
    let _ = k::<u64>;
    const _: () = {
        const _: () = {
            fn data_len<'test, T: flat_serialize::FlatSerializable<'test>>() {}
            let _ = data_len::<u32>;
            fn data<'test, T: flat_serialize::FlatSerializable<'test>>() {}
            let _ = data::<u8>;
        };
    };
    const _: () = {
        const _: () = {
            fn array<'test, T: flat_serialize::FlatSerializable<'test>>() {}
            let _ = array::<[u16; 3]>;
        };
    };
};
unsafe impl<'input> flat_serialize::FlatSerializable<'input> for BasicEnum<'input> {
    const REQUIRED_ALIGNMENT: usize = {
        use std::mem::align_of;
        let mut required_alignment: usize =
            <u64 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT;
        let alignment: usize = {
            let mut required_alignment =
                <u64 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT;
            let alignment = <u32 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT;
            if alignment > required_alignment {
                required_alignment = alignment;
            }
            let alignment = <u8 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT;
            if alignment > required_alignment {
                required_alignment = alignment;
            }
            required_alignment
        };
        if alignment > required_alignment {
            required_alignment = alignment;
        }
        let alignment: usize = {
            let mut required_alignment =
                <u64 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT;
            let alignment = <[u16; 3] as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT;
            if alignment > required_alignment {
                required_alignment = alignment;
            }
            required_alignment
        };
        if alignment > required_alignment {
            required_alignment = alignment;
        }
        required_alignment
    };
    const MAX_PROVIDED_ALIGNMENT: Option<usize> = {
        use std::mem::{align_of, size_of};
        let mut min_align: usize =
            match match <u64 as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT {
                Some(a) => Some(a),
                None => Some(8),
            } {
                None => 8,
                Some(align) => align,
            };
        let variant_alignment: usize = {
            let mut min_align: Option<usize> =
                match <u64 as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT {
                    Some(a) => Some(a),
                    None => Some(8),
                };
            let alignment = <u32 as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT;
            match (alignment, min_align) {
                (None, _) => (),
                (Some(align), None) => min_align = Some(align),
                (Some(align), Some(min)) if align < min => min_align = Some(align),
                _ => (),
            }
            let alignment = { Some(<u8 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT) };
            match (alignment, min_align) {
                (None, _) => (),
                (Some(align), None) => min_align = Some(align),
                (Some(align), Some(min)) if align < min => min_align = Some(align),
                _ => (),
            }
            let variant_size: usize = <u64 as flat_serialize::FlatSerializable>::MIN_LEN
                + <u32 as flat_serialize::FlatSerializable>::MIN_LEN
                + 0;
            let effective_alignment = match min_align {
                Some(align) => align,
                None => 8,
            };
            if variant_size % 8 == 0 && effective_alignment >= 8 {
                8
            } else if variant_size % 4 == 0 && effective_alignment >= 4 {
                4
            } else if variant_size % 2 == 0 && effective_alignment >= 2 {
                2
            } else {
                1
            }
        };
        if variant_alignment < min_align {
            min_align = variant_alignment
        }
        let variant_alignment: usize = {
            let mut min_align: Option<usize> =
                match <u64 as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT {
                    Some(a) => Some(a),
                    None => Some(8),
                };
            let alignment = <[u16; 3] as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT;
            match (alignment, min_align) {
                (None, _) => (),
                (Some(align), None) => min_align = Some(align),
                (Some(align), Some(min)) if align < min => min_align = Some(align),
                _ => (),
            }
            let variant_size: usize = <u64 as flat_serialize::FlatSerializable>::MIN_LEN
                + <[u16; 3] as flat_serialize::FlatSerializable>::MIN_LEN;
            let effective_alignment = match min_align {
                Some(align) => align,
                None => 8,
            };
            if variant_size % 8 == 0 && effective_alignment >= 8 {
                8
            } else if variant_size % 4 == 0 && effective_alignment >= 4 {
                4
            } else if variant_size % 2 == 0 && effective_alignment >= 2 {
                2
            } else {
                1
            }
        };
        if variant_alignment < min_align {
            min_align = variant_alignment
        }
        let min_size = Self::MIN_LEN;
        if min_size % 8 == 0 && min_align >= 8 {
            Some(8)
        } else if min_size % 4 == 0 && min_align >= 4 {
            Some(4)
        } else if min_size % 2 == 0 && min_align >= 2 {
            Some(2)
        } else {
            Some(1)
        }
    };
    const MIN_LEN: usize = {
        use std::mem::size_of;
        let mut size: Option<usize> = None;
        let variant_size = {
            let mut size: usize = <u64 as flat_serialize::FlatSerializable>::MIN_LEN;
            size += <u32 as flat_serialize::FlatSerializable>::MIN_LEN;
            size += 0;
            size
        };
        size = match size {
            None => Some(variant_size),
            Some(size) if size > variant_size => Some(variant_size),
            Some(size) => Some(size),
        };
        let variant_size = {
            let mut size: usize = <u64 as flat_serialize::FlatSerializable>::MIN_LEN;
            size += <[u16; 3] as flat_serialize::FlatSerializable>::MIN_LEN;
            size
        };
        size = match size {
            None => Some(variant_size),
            Some(size) if size > variant_size => Some(variant_size),
            Some(size) => Some(size),
        };
        match size {
            Some(size) => size,
            None => <u64 as flat_serialize::FlatSerializable>::MIN_LEN,
        }
    };
    const TRIVIAL_COPY: bool = false;
    type SLICE = flat_serialize::Iterable<'input, BasicEnum<'input>>;
    #[allow(unused_assignments, unused_variables)]
    #[inline(always)]
    unsafe fn try_ref(
        mut input: &'input [u8],
    ) -> Result<(Self, &'input [u8]), flat_serialize::WrapErr> {
        let __packet_macro_read_len = 0usize;
        let mut k = None;
        'tryref_tag: loop {
            {
                let (field, rem) = match <u64>::try_ref(input) {
                    Ok((f, b)) => (f, b),
                    Err(flat_serialize::WrapErr::InvalidTag(offset)) => {
                        return Err(flat_serialize::WrapErr::InvalidTag(
                            __packet_macro_read_len + offset,
                        ))
                    }
                    Err(..) => break 'tryref_tag,
                };
                input = rem;
                k = Some(field);
            };
            match k {
                Some(2) => {
                    let mut data_len: Option<u32> = None;
                    let mut data: Option<<u8 as flat_serialize::FlatSerializable<'_>>::SLICE> =
                        None;
                    'tryref_0: loop {
                        {
                            let (field, rem) = match <u32>::try_ref(input) {
                                Ok((f, b)) => (f, b),
                                Err(flat_serialize::WrapErr::InvalidTag(offset)) => {
                                    return Err(flat_serialize::WrapErr::InvalidTag(
                                        __packet_macro_read_len + offset,
                                    ))
                                }
                                Err(..) => break 'tryref_0,
                            };
                            input = rem;
                            data_len = Some(field);
                        }
                        {
                            let count = (data_len.clone().unwrap()) as usize;
                            let (field, rem) =
                                match <_ as flat_serialize::Slice<'_>>::try_ref(input, count) {
                                    Ok((f, b)) => (f, b),
                                    Err(flat_serialize::WrapErr::InvalidTag(offset)) => {
                                        return Err(flat_serialize::WrapErr::InvalidTag(
                                            __packet_macro_read_len + offset,
                                        ))
                                    }
                                    Err(..) => break 'tryref_0,
                                };
                            input = rem;
                            data = Some(field);
                        }
                        let _ref = BasicEnum::First {
                            data_len: data_len.unwrap(),
                            data: data.unwrap(),
                        };
                        return Ok((_ref, input));
                    }
                    return Err(flat_serialize::WrapErr::NotEnoughBytes(
                        std::mem::size_of::<u64>()
                            + <u32>::MIN_LEN
                            + (|| {
                                <u8>::MIN_LEN
                                    * (match data_len {
                                        Some(data_len) => data_len,
                                        None => return 0usize,
                                    }) as usize
                            })(),
                    ));
                }
                Some(3) => {
                    let mut array: Option<[u16; 3]> = None;
                    'tryref_1: loop {
                        {
                            let (field, rem) = match <[u16; 3]>::try_ref(input) {
                                Ok((f, b)) => (f, b),
                                Err(flat_serialize::WrapErr::InvalidTag(offset)) => {
                                    return Err(flat_serialize::WrapErr::InvalidTag(
                                        __packet_macro_read_len + offset,
                                    ))
                                }
                                Err(..) => break 'tryref_1,
                            };
                            input = rem;
                            array = Some(field);
                        }
                        let _ref = BasicEnum::Fixed {
                            array: array.unwrap(),
                        };
                        return Ok((_ref, input));
                    }
                    return Err(flat_serialize::WrapErr::NotEnoughBytes(
                        std::mem::size_of::<u64>() + <[u16; 3]>::MIN_LEN,
                    ));
                }
                _ => return Err(flat_serialize::WrapErr::InvalidTag(0)),
            }
        }
        Err(flat_serialize::WrapErr::NotEnoughBytes(
            ::std::mem::size_of::<u64>(),
        ))
    }
    #[allow(unused_assignments, unused_variables)]
    unsafe fn fill_slice<'out>(
        &self,
        input: &'out mut [std::mem::MaybeUninit<u8>],
    ) -> &'out mut [std::mem::MaybeUninit<u8>] {
        let total_len = self.len();
        let (mut input, rem) = input.split_at_mut(total_len);
        match self {
            BasicEnum::First { data_len, data } => {
                let k: &u64 = &2;
                unsafe {
                    input = k.fill_slice(input);
                }
                unsafe {
                    input = data_len.fill_slice(input);
                };
                unsafe {
                    let count = (*data_len) as usize;
                    input = <_ as flat_serialize::Slice<'_>>::fill_slice(data, count, input);
                }
            }
            BasicEnum::Fixed { array } => {
                let k: &u64 = &3;
                unsafe {
                    input = k.fill_slice(input);
                }
                unsafe {
                    input = array.fill_slice(input);
                }
            }
        }
        debug_assert_eq!(input.len(), 0);
        rem
    }
    #[allow(unused_assignments, unused_variables)]
    fn len(&self) -> usize {
        match self {
            BasicEnum::First { data_len, data } => {
                ::std::mem::size_of::<u64>()
                    + <u32 as flat_serialize::FlatSerializable>::len(data_len)
                    + (<_ as flat_serialize::Slice<'_>>::len(data, (*data_len) as usize))
            }
            BasicEnum::Fixed { array } => {
                ::std::mem::size_of::<u64>()
                    + <[u16; 3] as flat_serialize::FlatSerializable>::len(array)
            }
        }
    }
}
#[derive(Clone, Debug)]
pub enum PaddedEnum<'input> {
    First {
        padding: [u8; 3],
        data_len: u32,
        data: <u8 as flat_serialize::FlatSerializable<'input>>::SLICE,
    },
    Fixed {
        padding: u8,
        array: [u16; 3],
    },
}
#[allow(unused_assignments)]
const _: () = {
    use std::mem::{align_of, size_of};
    let mut current_size = 0;
    let mut min_align = 8;
    let _alignment_check: () =
        [()][(current_size) % <u8 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT];
    let _alignment_check2: () = [()]
        [(<u8 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT > min_align) as u8 as usize];
    current_size += <u8 as flat_serialize::FlatSerializable>::MIN_LEN;
    min_align = match <u8 as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT {
        Some(align) if align < min_align => align,
        _ => min_align,
    };
    {
        use std::mem::{align_of, size_of};
        let mut current_size = current_size;
        let mut min_align = min_align;
        let _alignment_check: () = [()]
            [(current_size) % <[u8; 3] as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT];
        let _alignment_check2: () = [()]
            [(<[u8; 3] as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT > min_align) as u8
                as usize];
        current_size += <[u8; 3] as flat_serialize::FlatSerializable>::MIN_LEN;
        min_align = match <[u8; 3] as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT {
            Some(align) if align < min_align => align,
            _ => min_align,
        };
        let _alignment_check: () =
            [()][(current_size) % <u32 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT];
        let _alignment_check2: () = [()]
            [(<u32 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT > min_align) as u8
                as usize];
        current_size += <u32 as flat_serialize::FlatSerializable>::MIN_LEN;
        min_align = match <u32 as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT {
            Some(align) if align < min_align => align,
            _ => min_align,
        };
        let _alignment_check: () =
            [()][(current_size) % <u8 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT];
        let _alignment_check2: () = [()]
            [(<u8 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT > min_align) as u8
                as usize];
        if <u8 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT < min_align {
            min_align = <u8 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT
        }
        min_align = match <u8 as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT {
            Some(align) if align < min_align => align,
            _ => min_align,
        };
    }
    {
        use std::mem::{align_of, size_of};
        let mut current_size = current_size;
        let mut min_align = min_align;
        let _alignment_check: () =
            [()][(current_size) % <u8 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT];
        let _alignment_check2: () = [()]
            [(<u8 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT > min_align) as u8
                as usize];
        current_size += <u8 as flat_serialize::FlatSerializable>::MIN_LEN;
        min_align = match <u8 as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT {
            Some(align) if align < min_align => align,
            _ => min_align,
        };
        let _alignment_check: () = [()]
            [(current_size) % <[u16; 3] as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT];
        let _alignment_check2: () = [()]
            [(<[u16; 3] as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT > min_align) as u8
                as usize];
        current_size += <[u16; 3] as flat_serialize::FlatSerializable>::MIN_LEN;
        min_align = match <[u16; 3] as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT {
            Some(align) if align < min_align => align,
            _ => min_align,
        };
    }
};
const _: () = {
    #[allow(dead_code)]
    enum UniquenessCheck {
        First = 2,
        Fixed = 3,
    }
};
const _: () = {
    fn k<'test, T: flat_serialize::FlatSerializable<'test>>() {}
    let _ = k::<u8>;
    const _: () = {
        const _: () = {
            fn padding<'test, T: flat_serialize::FlatSerializable<'test>>() {}
            let _ = padding::<[u8; 3]>;
            fn data_len<'test, T: flat_serialize::FlatSerializable<'test>>() {}
            let _ = data_len::<u32>;
            fn data<'test, T: flat_serialize::FlatSerializable<'test>>() {}
            let _ = data::<u8>;
        };
    };
    const _: () = {
        const _: () = {
            fn padding<'test, T: flat_serialize::FlatSerializable<'test>>() {}
            let _ = padding::<u8>;
            fn array<'test, T: flat_serialize::FlatSerializable<'test>>() {}
            let _ = array::<[u16; 3]>;
        };
    };
};
unsafe impl<'input> flat_serialize::FlatSerializable<'input> for PaddedEnum<'input> {
    const REQUIRED_ALIGNMENT: usize = {
        use std::mem::align_of;
        let mut required_alignment: usize =
            <u8 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT;
        let alignment: usize = {
            let mut required_alignment =
                <u8 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT;
            let alignment = <[u8; 3] as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT;
            if alignment > required_alignment {
                required_alignment = alignment;
            }
            let alignment = <u32 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT;
            if alignment > required_alignment {
                required_alignment = alignment;
            }
            let alignment = <u8 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT;
            if alignment > required_alignment {
                required_alignment = alignment;
            }
            required_alignment
        };
        if alignment > required_alignment {
            required_alignment = alignment;
        }
        let alignment: usize = {
            let mut required_alignment =
                <u8 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT;
            let alignment = <u8 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT;
            if alignment > required_alignment {
                required_alignment = alignment;
            }
            let alignment = <[u16; 3] as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT;
            if alignment > required_alignment {
                required_alignment = alignment;
            }
            required_alignment
        };
        if alignment > required_alignment {
            required_alignment = alignment;
        }
        required_alignment
    };
    const MAX_PROVIDED_ALIGNMENT: Option<usize> = {
        use std::mem::{align_of, size_of};
        let mut min_align: usize =
            match match <u8 as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT {
                Some(a) => Some(a),
                None => Some(8),
            } {
                None => 8,
                Some(align) => align,
            };
        let variant_alignment: usize = {
            let mut min_align: Option<usize> =
                match <u8 as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT {
                    Some(a) => Some(a),
                    None => Some(8),
                };
            let alignment = <[u8; 3] as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT;
            match (alignment, min_align) {
                (None, _) => (),
                (Some(align), None) => min_align = Some(align),
                (Some(align), Some(min)) if align < min => min_align = Some(align),
                _ => (),
            }
            let alignment = <u32 as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT;
            match (alignment, min_align) {
                (None, _) => (),
                (Some(align), None) => min_align = Some(align),
                (Some(align), Some(min)) if align < min => min_align = Some(align),
                _ => (),
            }
            let alignment = { Some(<u8 as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT) };
            match (alignment, min_align) {
                (None, _) => (),
                (Some(align), None) => min_align = Some(align),
                (Some(align), Some(min)) if align < min => min_align = Some(align),
                _ => (),
            }
            let variant_size: usize = <u8 as flat_serialize::FlatSerializable>::MIN_LEN
                + <[u8; 3] as flat_serialize::FlatSerializable>::MIN_LEN
                + <u32 as flat_serialize::FlatSerializable>::MIN_LEN
                + 0;
            let effective_alignment = match min_align {
                Some(align) => align,
                None => 8,
            };
            if variant_size % 8 == 0 && effective_alignment >= 8 {
                8
            } else if variant_size % 4 == 0 && effective_alignment >= 4 {
                4
            } else if variant_size % 2 == 0 && effective_alignment >= 2 {
                2
            } else {
                1
            }
        };
        if variant_alignment < min_align {
            min_align = variant_alignment
        }
        let variant_alignment: usize = {
            let mut min_align: Option<usize> =
                match <u8 as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT {
                    Some(a) => Some(a),
                    None => Some(8),
                };
            let alignment = <u8 as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT;
            match (alignment, min_align) {
                (None, _) => (),
                (Some(align), None) => min_align = Some(align),
                (Some(align), Some(min)) if align < min => min_align = Some(align),
                _ => (),
            }
            let alignment = <[u16; 3] as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT;
            match (alignment, min_align) {
                (None, _) => (),
                (Some(align), None) => min_align = Some(align),
                (Some(align), Some(min)) if align < min => min_align = Some(align),
                _ => (),
            }
            let variant_size: usize = <u8 as flat_serialize::FlatSerializable>::MIN_LEN
                + <u8 as flat_serialize::FlatSerializable>::MIN_LEN
                + <[u16; 3] as flat_serialize::FlatSerializable>::MIN_LEN;
            let effective_alignment = match min_align {
                Some(align) => align,
                None => 8,
            };
            if variant_size % 8 == 0 && effective_alignment >= 8 {
                8
            } else if variant_size % 4 == 0 && effective_alignment >= 4 {
                4
            } else if variant_size % 2 == 0 && effective_alignment >= 2 {
                2
            } else {
                1
            }
        };
        if variant_alignment < min_align {
            min_align = variant_alignment
        }
        let min_size = Self::MIN_LEN;
        if min_size % 8 == 0 && min_align >= 8 {
            Some(8)
        } else if min_size % 4 == 0 && min_align >= 4 {
            Some(4)
        } else if min_size % 2 == 0 && min_align >= 2 {
            Some(2)
        } else {
            Some(1)
        }
    };
    const MIN_LEN: usize = {
        use std::mem::size_of;
        let mut size: Option<usize> = None;
        let variant_size = {
            let mut size: usize = <u8 as flat_serialize::FlatSerializable>::MIN_LEN;
            size += <[u8; 3] as flat_serialize::FlatSerializable>::MIN_LEN;
            size += <u32 as flat_serialize::FlatSerializable>::MIN_LEN;
            size += 0;
            size
        };
        size = match size {
            None => Some(variant_size),
            Some(size) if size > variant_size => Some(variant_size),
            Some(size) => Some(size),
        };
        let variant_size = {
            let mut size: usize = <u8 as flat_serialize::FlatSerializable>::MIN_LEN;
            size += <u8 as flat_serialize::FlatSerializable>::MIN_LEN;
            size += <[u16; 3] as flat_serialize::FlatSerializable>::MIN_LEN;
            size
        };
        size = match size {
            None => Some(variant_size),
            Some(size) if size > variant_size => Some(variant_size),
            Some(size) => Some(size),
        };
        match size {
            Some(size) => size,
            None => <u8 as flat_serialize::FlatSerializable>::MIN_LEN,
        }
    };
    const TRIVIAL_COPY: bool = false;
    type SLICE = flat_serialize::Iterable<'input, PaddedEnum<'input>>;
    #[allow(unused_assignments, unused_variables)]
    #[inline(always)]
    unsafe fn try_ref(
        mut input: &'input [u8],
    ) -> Result<(Self, &'input [u8]), flat_serialize::WrapErr> {
        let __packet_macro_read_len = 0usize;
        let mut k = None;
        'tryref_tag: loop {
            {
                let (field, rem) = match <u8>::try_ref(input) {
                    Ok((f, b)) => (f, b),
                    Err(flat_serialize::WrapErr::InvalidTag(offset)) => {
                        return Err(flat_serialize::WrapErr::InvalidTag(
                            __packet_macro_read_len + offset,
                        ))
                    }
                    Err(..) => break 'tryref_tag,
                };
                input = rem;
                k = Some(field);
            };
            match k {
                Some(2) => {
                    let mut padding: Option<[u8; 3]> = None;
                    let mut data_len: Option<u32> = None;
                    let mut data: Option<<u8 as flat_serialize::FlatSerializable<'_>>::SLICE> =
                        None;
                    'tryref_0: loop {
                        {
                            let (field, rem) = match <[u8; 3]>::try_ref(input) {
                                Ok((f, b)) => (f, b),
                                Err(flat_serialize::WrapErr::InvalidTag(offset)) => {
                                    return Err(flat_serialize::WrapErr::InvalidTag(
                                        __packet_macro_read_len + offset,
                                    ))
                                }
                                Err(..) => break 'tryref_0,
                            };
                            input = rem;
                            padding = Some(field);
                        }
                        {
                            let (field, rem) = match <u32>::try_ref(input) {
                                Ok((f, b)) => (f, b),
                                Err(flat_serialize::WrapErr::InvalidTag(offset)) => {
                                    return Err(flat_serialize::WrapErr::InvalidTag(
                                        __packet_macro_read_len + offset,
                                    ))
                                }
                                Err(..) => break 'tryref_0,
                            };
                            input = rem;
                            data_len = Some(field);
                        }
                        {
                            let count = (data_len.clone().unwrap()) as usize;
                            let (field, rem) =
                                match <_ as flat_serialize::Slice<'_>>::try_ref(input, count) {
                                    Ok((f, b)) => (f, b),
                                    Err(flat_serialize::WrapErr::InvalidTag(offset)) => {
                                        return Err(flat_serialize::WrapErr::InvalidTag(
                                            __packet_macro_read_len + offset,
                                        ))
                                    }
                                    Err(..) => break 'tryref_0,
                                };
                            input = rem;
                            data = Some(field);
                        }
                        let _ref = PaddedEnum::First {
                            padding: padding.unwrap(),
                            data_len: data_len.unwrap(),
                            data: data.unwrap(),
                        };
                        return Ok((_ref, input));
                    }
                    return Err(flat_serialize::WrapErr::NotEnoughBytes(
                        std::mem::size_of::<u8>()
                            + <[u8; 3]>::MIN_LEN
                            + <u32>::MIN_LEN
                            + (|| {
                                <u8>::MIN_LEN
                                    * (match data_len {
                                        Some(data_len) => data_len,
                                        None => return 0usize,
                                    }) as usize
                            })(),
                    ));
                }
                Some(3) => {
                    let mut padding: Option<u8> = None;
                    let mut array: Option<[u16; 3]> = None;
                    'tryref_1: loop {
                        {
                            let (field, rem) = match <u8>::try_ref(input) {
                                Ok((f, b)) => (f, b),
                                Err(flat_serialize::WrapErr::InvalidTag(offset)) => {
                                    return Err(flat_serialize::WrapErr::InvalidTag(
                                        __packet_macro_read_len + offset,
                                    ))
                                }
                                Err(..) => break 'tryref_1,
                            };
                            input = rem;
                            padding = Some(field);
                        }
                        {
                            let (field, rem) = match <[u16; 3]>::try_ref(input) {
                                Ok((f, b)) => (f, b),
                                Err(flat_serialize::WrapErr::InvalidTag(offset)) => {
                                    return Err(flat_serialize::WrapErr::InvalidTag(
                                        __packet_macro_read_len + offset,
                                    ))
                                }
                                Err(..) => break 'tryref_1,
                            };
                            input = rem;
                            array = Some(field);
                        }
                        let _ref = PaddedEnum::Fixed {
                            padding: padding.unwrap(),
                            array: array.unwrap(),
                        };
                        return Ok((_ref, input));
                    }
                    return Err(flat_serialize::WrapErr::NotEnoughBytes(
                        std::mem::size_of::<u8>() + <u8>::MIN_LEN + <[u16; 3]>::MIN_LEN,
                    ));
                }
                _ => return Err(flat_serialize::WrapErr::InvalidTag(0)),
            }
        }
        Err(flat_serialize::WrapErr::NotEnoughBytes(
            ::std::mem::size_of::<u8>(),
        ))
    }
    #[allow(unused_assignments, unused_variables)]
    unsafe fn fill_slice<'out>(
        &self,
        input: &'out mut [std::mem::MaybeUninit<u8>],
    ) -> &'out mut [std::mem::MaybeUninit<u8>] {
        let total_len = self.len();
        let (mut input, rem) = input.split_at_mut(total_len);
        match self {
            PaddedEnum::First {
                padding,
                data_len,
                data,
            } => {
                let k: &u8 = &2;
                unsafe {
                    input = k.fill_slice(input);
                }
                unsafe {
                    input = padding.fill_slice(input);
                };
                unsafe {
                    input = data_len.fill_slice(input);
                };
                unsafe {
                    let count = (*data_len) as usize;
                    input = <_ as flat_serialize::Slice<'_>>::fill_slice(data, count, input);
                }
            }
            PaddedEnum::Fixed { padding, array } => {
                let k: &u8 = &3;
                unsafe {
                    input = k.fill_slice(input);
                }
                unsafe {
                    input = padding.fill_slice(input);
                };
                unsafe {
                    input = array.fill_slice(input);
                }
            }
        }
        debug_assert_eq!(input.len(), 0);
        rem
    }
    #[allow(unused_assignments, unused_variables)]
    fn len(&self) -> usize {
        match self {
            PaddedEnum::First {
                padding,
                data_len,
                data,
            } => {
                ::std::mem::size_of::<u8>()
                    + <[u8; 3] as flat_serialize::FlatSerializable>::len(padding)
                    + <u32 as flat_serialize::FlatSerializable>::len(data_len)
                    + (<_ as flat_serialize::Slice<'_>>::len(data, (*data_len) as usize))
            }
            PaddedEnum::Fixed { padding, array } => {
                ::std::mem::size_of::<u8>()
                    + <u8 as flat_serialize::FlatSerializable>::len(padding)
                    + <[u16; 3] as flat_serialize::FlatSerializable>::len(array)
            }
        }
    }
}
