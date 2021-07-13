#[macro_export]
macro_rules! pg_type {
    // base case, all fields are collected into $vals
    (
        $(#[$attrs: meta])*
        struct $name: ident $(<$inlife: lifetime>)? {
            $(,)?
        }

        $(%($($vals:tt)*))?
    ) => {
        $crate::pg_type_impl!{
            'input
            $(#[$attrs])*
            struct $name $(<$inlife>)? {
                $($($vals)*)?
            }
        }
    };
    // eat a struct field and add it to $vals
    (
        $(#[$attrs: meta])*
        struct $name: ident $(<$inlife: lifetime>)? {
            $(#[$fattrs: meta])* $field:ident : $typ: tt $(<$life:lifetime>)?,
            $($tail: tt)*
        }

        $(%($($vals:tt)*))?
    ) => {
        $crate::pg_type!{
            $(#[$attrs])*
            struct $name $(<$inlife>)? {
                $($tail)*
            }

            %( $($($vals)*)?
                $(#[$fattrs])* $field : $typ $(<$life>)? ,
            )
        }
    };
    // eat an enum field, define the enum, and add the equivalent struct field to $vals
    (
        $(#[$attrs: meta])*
        struct $name: ident $(<$inlife: lifetime>)? {
            $(#[$fattrs: meta])* $estructfield: ident : $(#[$enumattrs: meta])* enum $ename:ident $(<$elife: lifetime>)? {
                $($enum_def:tt)*
            },
            $($tail: tt)*
        }

        $(%($($vals:tt)*))?
    ) => {
        flat_serialize_macro::flat_serialize! {
            $(#[$attrs])*
            $(#[$enumattrs])*
            #[derive(serde::Serialize, serde::Deserialize)]
            #[flat_serialize::field_attr(
                fixed = r##"#[serde(deserialize_with = "crate::serialization::serde_reference_adaptor::deserialize")]"##,
                variable = r##"#[serde(deserialize_with = "crate::serialization::serde_reference_adaptor::deserialize_slice")]"##,
            )]
            enum $ename $(<$elife>)? {
                $($enum_def)*
            }
        }
        $crate::pg_type!{
            $(#[$attrs])*
            struct $name $(<$inlife>)? {
                $($tail)*
            }

            %( $($($vals)*)?
                $(#[$fattrs])*
                #[flat_serialize::flatten]
                $estructfield : $ename $(<$elife>)?,
            )
        }
    }
}

#[macro_export]
macro_rules! pg_type_impl {
    (
        $lifetemplate: lifetime
        $(#[$attrs: meta])*
        struct $name: ident $(<$inlife: lifetime>)? {
            $($(#[$fattrs: meta])* $field:ident : $typ: tt $(<$life:lifetime>)?),*
            $(,)?
        }
    ) => {
        ::paste::paste! {
            $(#[$attrs])*
            #[derive(pgx::PostgresType, Clone)]
            #[inoutfuncs]
            pub struct $name<$lifetemplate>([<$name Data>] $(<$inlife>)?, Option<&$lifetemplate [u8]>);

            flat_serialize_macro::flat_serialize! {
                $(#[$attrs])*
                #[derive(serde::Serialize, serde::Deserialize)]
                #[flat_serialize::field_attr(
                    fixed = r##"#[serde(deserialize_with = "crate::serialization::serde_reference_adaptor::deserialize")]"##,
                    variable = r##"#[serde(deserialize_with = "crate::serialization::serde_reference_adaptor::deserialize_slice")]"##,
                )]
                struct [<$name Data>] $(<$inlife>)? {
                    #[serde(skip, default="crate::serialization::serde_reference_adaptor::default_header")]
                    header: u32,
                    version: u8,
                    #[serde(skip, default="crate::serialization::serde_reference_adaptor::default_padding")]
                    padding: [u8; 3],
                    $($(#[$fattrs])* $field: $typ $(<$life>)?),*
                }
            }

            impl<'input> $name<'input> {
                pub fn in_current_context<'foo>(&self) -> $name<'foo> {
                    unsafe { self.0.flatten() }
                }
            }

            impl<$lifetemplate> [<$name Data>] $(<$inlife>)? {
                pub unsafe fn flatten<'any>(&self) -> $name<'any> {
                    let bytes: &'static [u8] = self.to_pg_bytes();
                    let wrapped = [<$name Data>]::try_ref(bytes).unwrap().0;
                    $name(wrapped, Some(bytes))
                }

                pub fn to_pg_bytes(&self) -> &'static [u8] {
                    let mut output = vec![];
                    self.fill_vec(&mut output);
                    unsafe {
                        ::pgx::set_varsize(output.as_mut_ptr() as *mut _, output.len() as i32);
                    }
                    &*output.leak()
                }
            }

            impl<$lifetemplate> pgx::FromDatum for $name<$lifetemplate> {
                unsafe fn from_datum(datum: pgx::pg_sys::Datum, is_null: bool, _: pg_sys::Oid) -> Option<Self>
                where
                    Self: Sized,
                {
                    if is_null {
                        return None;
                    }

                    let mut ptr = pg_sys::pg_detoast_datum_packed(datum as *mut pg_sys::varlena);
                    //TODO is there a better way to do this?
                    if pgx::varatt_is_1b(ptr) {
                        ptr = pg_sys::pg_detoast_datum_copy(ptr);
                    }
                    let data_len = pgx::varsize_any(ptr);
                    let bytes = std::slice::from_raw_parts(ptr as *mut u8, data_len);
                    let (data, _) = match [<$name Data>]::try_ref(bytes) {
                        Ok(wrapped) => wrapped,
                        Err(e) => error!(concat!("invalid ", stringify!($name), " {:?}, got len {}"), e, bytes.len()),
                    };

                    $name(data, Some(bytes)).into()
                }
            }

            impl<$lifetemplate> pgx::IntoDatum for $name<$lifetemplate> {
                fn into_datum(self) -> Option<pgx::pg_sys::Datum> {
                    let datum = match self.1 {
                        Some(bytes) => bytes.as_ptr() as pgx::pg_sys::Datum,
                        None => self.0.to_pg_bytes().as_ptr() as pgx::pg_sys::Datum,
                    };
                    Some(datum)
                }

                fn type_oid() -> pg_sys::Oid {
                    rust_regtypein::<Self>()
                }
            }

            impl<$lifetemplate> ::std::ops::Deref for $name <$lifetemplate> {
                type Target=[<$name Data>] $(<$inlife>)?;
                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }

            impl<$lifetemplate> From<[<$name Data>]$(<$inlife>)?> for $name<$lifetemplate> {
                fn from(inner: [<$name Data>]$(<$inlife>)?) -> Self {
                    Self(inner, None)
                }
            }

            impl<$lifetemplate> From<[<$name Data>]$(<$inlife>)?> for Option<$name<$lifetemplate>> {
                fn from(inner: [<$name Data>]$(<$inlife>)?) -> Self {
                    Some($name(inner, None))
                }
            }
        }
    }
}

#[macro_export]
macro_rules! json_inout_funcs {
    ($name:ident) => {
        impl<'input> InOutFuncs for $name<'input> {
            fn output(&self, buffer: &mut StringInfo) {
                use $crate::serialization::{EncodedStr::*, str_to_db_encoding};

                let stringified = serde_json::to_string(&**self).unwrap();
                match str_to_db_encoding(&stringified) {
                    Utf8(s) => buffer.push_str(s),
                    Other(s) => buffer.push_bytes(s.to_bytes()),
                }
            }

            fn input(input: &std::ffi::CStr) -> $name<'input>
            where
                Self: Sized,
            {
                use $crate::serialization::str_from_db_encoding;

                // SAFETY our serde shims will allocate and leak copies of all
                // the data, so the lifetimes of the borrows aren't actually
                // relevant to the output lifetime
                let val = unsafe {
                    unsafe fn extend_lifetime(s: &str) -> &'static str {
                        std::mem::transmute(s)
                    }
                    let input = extend_lifetime(str_from_db_encoding(input));
                    serde_json::from_str(input).unwrap()
                };
                unsafe { Self(val, None).flatten() }
            }
        }
    };
}

#[macro_export]
macro_rules! flatten {
    ($typ:ident { $($field:ident: $value:expr),* $(,)? }) => {
        {
            let data = ::paste::paste! {
                [<$typ Data>] {
                    header: 0,
                    version: 1,
                    padding: [0; 3],
                    $(
                        $field: $value
                    ),*
                }
            };
            data.flatten()
        }
    }
}

#[repr(u8)]
pub enum SerializationType {
    Default = 1,
}

#[macro_export]
macro_rules! do_serialize {
    ($state: ident) => {
        {
            $crate::do_serialize!($state, version: 1)
        }
    };
    ($state: ident, version: $version: expr) => {
        {
            use $crate::type_builder::SerializationType;

            let state = &*$state;
            let serialized_size = bincode::serialized_size(state)
                .unwrap_or_else(|e| pgx::error!("serialization error {}", e));
            let size = serialized_size + 2; // size of serialized data + our version flags
            let mut bytes = Vec::with_capacity(size as usize + 4);
            let varsize = [0; 4];
            bytes.extend_from_slice(&varsize);
            // type version
            bytes.push($version);
            // serialization version; 1 for bincode is currently the only option
            bytes.push(SerializationType::Default as u8);
            bincode::serialize_into(&mut bytes, state)
                .unwrap_or_else(|e| pgx::error!("serialization error {}", e));
            unsafe {
                ::pgx::set_varsize(bytes.as_mut_ptr() as *mut _, bytes.len() as i32);
            }
            bytes.leak().as_mut_ptr() as pg_sys::Datum
        }
    };
}
#[macro_export]
macro_rules! do_deserialize {
    ($bytes: ident, $t: ty) => {
        {
            use $crate::type_builder::SerializationType;

            let state: $t = unsafe {
                let detoasted = pg_sys::pg_detoast_datum_packed($bytes as *mut _);
                let len = pgx::varsize_any_exhdr(detoasted);
                let data = pgx::vardata_any(detoasted);
                let bytes = slice::from_raw_parts(data as *mut u8, len);
                if bytes.len() < 1 {
                    pgx::error!("deserialization error, no bytes")
                }
                if bytes[0] != 1 {
                    pgx::error!("deserialization error, invalid serialization version {}", bytes[0])
                }
                if bytes[1] != SerializationType::Default as u8 {
                    pgx::error!("deserialization error, invalid serialization type {}", bytes[1])
                }
                bincode::deserialize(&bytes[2..]).unwrap_or_else(|e|
                    pgx::error!("deserialization error {}", e))
            };
            state.into()
        }
    };
}
