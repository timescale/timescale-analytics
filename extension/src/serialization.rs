
use std::{convert::TryInto, ffi::CStr, os::raw::{c_char, c_int}};
pub use self::types::{PgTypId, ShortTypeId};
pub use self::collations::PgCollationId;

use pgx::pg_sys;

mod types;
mod collations;

// basically timestamptz_out
#[no_mangle]
pub extern "C" fn _ts_toolkit_encode_timestamptz(
    dt: pg_sys::TimestampTz, buf: &mut [c_char; pg_sys::MAXDATELEN as _]
) {
    let mut tz: c_int = 0;
    let mut tt: pg_sys::pg_tm = unsafe{ std::mem::MaybeUninit::zeroed().assume_init() };
    let mut fsec = 0;
    let mut tzn = std::ptr::null();
    unsafe {
        if dt == pg_sys::TimestampTz::MAX || dt == pg_sys::TimestampTz::MIN {
            return pg_sys::EncodeSpecialTimestamp(dt, buf.as_mut_ptr())
        }
        let err = pg_sys::timestamp2tm(dt, &mut tz, &mut tt, &mut fsec, &mut tzn, std::ptr::null_mut());
        if err != 0 {
            panic!("timestamp out of range")
        }
        pg_sys::EncodeDateTime(&mut tt, fsec, true, tz, tzn, pg_sys::DateStyle, buf.as_mut_ptr())
    }
}

#[no_mangle]
 // this is only going to be used to communicate with a rust lib we compile with this one
#[allow(improper_ctypes_definitions)]
pub extern "C" fn _ts_toolkit_decode_timestamptz(
    text: &str
) -> i64 {
    use std::{ffi::CString, mem::MaybeUninit, ptr};
    let str = CString::new(text).unwrap();
    unsafe {
        let mut fsec = 0;
        let mut tt = MaybeUninit::zeroed().assume_init();
        let tm = &mut tt;
        let mut tz = 0;
        let mut dtype = 0;
        let mut nf = 0;
        let mut field = [ptr::null_mut(); pg_sys::MAXDATEFIELDS as _];
        let mut ftype = [0; pg_sys::MAXDATEFIELDS as _];
        let mut workbuf = [0; pg_sys::MAXDATELEN as usize + pg_sys::MAXDATEFIELDS as usize];
        let mut dterr = pg_sys::ParseDateTime(str.as_ptr(), workbuf.as_mut_ptr(), workbuf.len(), field.as_mut_ptr(), ftype.as_mut_ptr(), pg_sys::MAXDATEFIELDS as i32, &mut nf);
        if dterr == 0 {
            dterr = pg_sys::DecodeDateTime(field.as_mut_ptr(), ftype.as_mut_ptr(), nf, &mut dtype, tm, &mut fsec, &mut tz)
        }
        if dterr != 0 {
            pg_sys::DateTimeParseError(dterr, str.as_ptr(), b"timestamptz\0".as_ptr().cast::<c_char>());
            return 0
        }

        match dtype as u32 {
            pg_sys::DTK_DATE => {
                let mut result = 0;
                let err = pg_sys::tm2timestamp(tm, fsec, &mut tz, &mut result);
                if err != 0 {
                    // TODO pgx error with correct errcode?
                    panic!("timestamptz \"{}\" out of range", text)
                }
                result
            },
            pg_sys::DTK_EPOCH => pg_sys::SetEpochTimestamp(),
            pg_sys::DTK_LATE => pg_sys::TimestampTz::MAX,
            pg_sys::DTK_EARLY => pg_sys::TimestampTz::MIN,
            _ => panic!("unexpected result {} when parsing timestamptz \"{}\"", dtype, text),
        }
    }
}

// FIXME upstream to pgx
pub(crate) const PG_UTF8: i32 = 6;
extern "C" {
    pub fn pg_server_to_any(s: *const c_char, len: c_int, encoding: c_int) -> *const c_char;
    pub fn pg_any_to_server(s: *const c_char, len: c_int, encoding: c_int) -> *const c_char;
    pub fn GetDatabaseEncoding() -> c_int;
}

pub enum EncodedStr<'s> {
    Utf8(&'s str),
    Other(&'s CStr)
}

pub fn str_to_db_encoding(s: &str) -> EncodedStr {
    if unsafe { GetDatabaseEncoding() == PG_UTF8 } {
        return EncodedStr::Utf8(s)
    }

    let bytes = s.as_bytes();
    let encoded = unsafe {
        pg_any_to_server(bytes.as_ptr() as *const c_char, bytes.len().try_into().unwrap(), PG_UTF8)
    };
    if encoded as usize == bytes.as_ptr() as usize {
        return EncodedStr::Utf8(s)
    }

    let cstr = unsafe { CStr::from_ptr(encoded) };
    return EncodedStr::Other(cstr)
}

pub fn str_from_db_encoding(s: &CStr) -> &str {
    if unsafe { GetDatabaseEncoding() == PG_UTF8 } {
        return s.to_str().unwrap()
    }

    let str_len = s.to_bytes().len().try_into().unwrap();
    let encoded = unsafe {
        pg_server_to_any(s.as_ptr(), str_len, PG_UTF8)
    };
    if encoded as usize == s.as_ptr() as usize {
        //TODO redundant check?
        return s.to_str().unwrap()
    }
    return unsafe { CStr::from_ptr(encoded).to_str().unwrap() }
}

// NOTE this module assumes that the rust allocator is the postgres allocator,
//      and thus leaks memory assuming that MemoryContext deletion will clean
//      it up
pub(crate) mod serde_reference_adaptor {
    use serde::{Deserialize, Deserializer};

    pub(crate) fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where D: Deserializer<'de>, T: Deserialize<'de> {
        T::deserialize(deserializer)
    }

    pub(crate) fn deserialize_slice<'de, D, S>(deserializer: D) -> Result<S, D::Error>
    where D: Deserializer<'de>, S: LeakableSlice<'de> {
        S::deserialize_slice(deserializer)
    }

    pub(crate) trait LeakableSlice<'de>: Sized {
        fn deserialize_slice<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>;
    }

    impl<'de, T> LeakableSlice<'de> for &'static [T]
    where T: Deserialize<'de> {
        fn deserialize_slice<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de> {
            let boxed = <Box<[T]>>::deserialize(deserializer)?.into();
            Ok(Box::leak(boxed))
        }
    }

    impl<'de, T> LeakableSlice<'de> for flat_serialize::Iterable<'static, T>
    where T: Deserialize<'de> {
        fn deserialize_slice<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de> {
            Self::deserialize(deserializer)
        }
    }

    pub(crate) fn default_padding() -> [u8; 3] {
        [0; 3]
    }

    pub(crate) fn default_header() -> u32 {
        0
    }
}
