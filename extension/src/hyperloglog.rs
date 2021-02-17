use std::{
    convert::TryInto,
    hash::{BuildHasher, Hasher},
    mem::size_of,
    slice,
};

use serde::{Deserialize, Serialize};

use pg_sys::{Datum, Oid};
use pgx::*;

use flat_serialize::*;

use crate::{
    aggregate_utils::{get_collation, in_aggregate_context},
    flatten, json_inout_funcs,
    palloc::Internal,
    pg_type,
    serialization::{PgCollationId, ShortTypeId},
};

use hyperloglogplusplus::{HyperLogLog as HLL, HyperLogLogStorage};

#[derive(Serialize, Deserialize, Clone)]
pub struct HyperLogLogTrans {
    logger: HLL<'static, Datum, DatumHashBuilder>,
}

#[allow(non_camel_case_types)]
type int = i32;
type AnyElement = Datum;

#[pg_extern(schema = "toolkit_experimental")]
pub fn hyperloglog_trans(
    state: Option<Internal<HyperLogLogTrans>>,
    size: int,
    value: Option<AnyElement>,
    fc: pg_sys::FunctionCallInfo,
) -> Option<Internal<HyperLogLogTrans>> {
    unsafe {
        in_aggregate_context(fc, || {
            //TODO is this the right way to handle NULL?
            let value = match value {
                None => return state,
                Some(value) => value,
            };
            let mut state = match state {
                None => {
                    // TODO specialize hash function for bytea types?
                    //      ints? floats? uuids? other primitive types?
                    let size: usize = size.try_into().unwrap();
                    let b = size.checked_next_power_of_two().unwrap().trailing_zeros();
                    let typ = pgx::get_getarg_type(fc, 2);
                    let collation = get_collation(fc);
                    let hasher = DatumHashBuilder::from_type_id(typ, collation);
                    let trans = HyperLogLogTrans {
                        logger: HLL::new(b as u8, hasher),
                    };
                    trans.into()
                }
                Some(state) => state,
            };
            state.logger.add(&value);
            Some(state)
        })
    }
}

#[pg_extern(schema = "toolkit_experimental")]
pub fn hyperloglog_combine(
    state1: Option<Internal<HyperLogLogTrans>>,
    state2: Option<Internal<HyperLogLogTrans>>,
    fcinfo: pg_sys::FunctionCallInfo,
) -> Option<Internal<HyperLogLogTrans>> {
    unsafe {
        in_aggregate_context(fcinfo, || match (state1, state2) {
            (None, None) => None,
            (None, Some(state2)) => Some(state2.clone().into()),
            (Some(state1), None) => Some(state1.clone().into()),
            (Some(state1), Some(state2)) => {
                let mut logger = state1.logger.clone();
                logger.merge_in(&state2.logger);
                Some(HyperLogLogTrans { logger }.into())
            }
        })
    }
}

#[allow(non_camel_case_types)]
type bytea = pg_sys::Datum;

#[pg_extern(schema = "toolkit_experimental")]
pub fn hyperloglog_serialize(state: Internal<HyperLogLogTrans>) -> bytea {
    crate::do_serialize!(state)
}

#[pg_extern(schema = "toolkit_experimental", strict)]
pub fn hyperloglog_deserialize(
    bytes: bytea,
    _internal: Option<Internal<()>>,
) -> Internal<HyperLogLogTrans> {
    crate::do_deserialize!(bytes, HyperLogLogTrans)
}

pg_type! {
    #[derive(Debug)]
    struct HyperLogLog<'input> {
        #[flat_serialize::flatten]
        log: Storage<'input>,
    }
}

flat_serialize_macro::flat_serialize! {
    #[derive(Debug, Serialize, Deserialize)]
    #[flat_serialize::field_attr(
        fixed = r##"#[serde(deserialize_with = "crate::serialization::serde_reference_adaptor::deserialize")]"##,
        variable = r##"#[serde(deserialize_with = "crate::serialization::serde_reference_adaptor::deserialize_slice")]"##,
    )]
    enum Storage<'a> {
        storage_kind: u64,
        Sparse: 1 {
            num_compressed: u64,
            // Oids are stored in postgres arrays, so it should be safe to store them
            // in our types as long as we do send/recv and in/out correctly
            // see https://github.com/postgres/postgres/blob/b8d0cda53377515ac61357ec4a60e85ca873f486/src/include/utils/array.h#L90
            element_type: ShortTypeId,
            collation: PgCollationId,
            compressed_bytes: u32,
            precision: u8,
            compressed: [u8; self.compressed_bytes],
        },
        Dense: 2 {
            // Oids are stored in postgres arrays, so it should be safe to store them
            // in our types as long as we do send/recv and in/out correctly
            // see https://github.com/postgres/postgres/blob/b8d0cda53377515ac61357ec4a60e85ca873f486/src/include/utils/array.h#L90
            element_type: ShortTypeId,
            collation: PgCollationId,
            precision: u8,
            registers: [u8; 1 + (1usize << self.precision) * 6 / 8] //TODO should we just store len?
        },
    }
}

// hack to allow us to qualify names with "toolkit_experimental"
// so that pgx generates the correct SQL
mod toolkit_experimental {
    pub(crate) use super::*;

    varlena_type!(Hyperloglog);
}

json_inout_funcs!(HyperLogLog);

#[pg_extern(schema = "toolkit_experimental")]
fn hyperloglog_final(
    state: Option<Internal<HyperLogLogTrans>>,
    fcinfo: pg_sys::FunctionCallInfo,
) -> Option<toolkit_experimental::HyperLogLog<'static>> {
    unsafe {
        in_aggregate_context(fcinfo, || {
            let mut state = match state {
                None => return None,
                Some(state) => state,
            };

            flatten_log(&mut state.logger).into()
        })
    }
}

extension_sql!(
    r#"
CREATE AGGREGATE toolkit_experimental.hyperloglog(size int, value AnyElement)
(
    stype = internal,
    sfunc = toolkit_experimental.hyperloglog_trans,
    finalfunc = toolkit_experimental.hyperloglog_final,
    combinefunc = toolkit_experimental.hyperloglog_combine,
    serialfunc = toolkit_experimental.hyperloglog_serialize,
    deserialfunc = toolkit_experimental.hyperloglog_deserialize
);
"#
);

#[pg_extern(schema = "toolkit_experimental")]
pub fn hyperloglog_count<'input>(
    hyperloglog: toolkit_experimental::HyperLogLog<'input>
) -> i64 {
    // count does not depend on the type parameters
    let log = match hyperloglog.log {
        Storage::Sparse { num_compressed, precision, compressed, .. } =>
            HLL::<Datum, ()>::from_sparse_parts(compressed, num_compressed, precision, ()),
        Storage::Dense { precision, registers, .. } =>
        HLL::<Datum, ()>::from_dense_parts(registers, precision, ()),
    };
    log.immutable_estimate_count() as i64
}

#[pg_extern(name="rollup", schema = "toolkit_experimental")]
pub fn hyperloglog_union<'input>(
    a: toolkit_experimental::HyperLogLog<'input>,
    b: toolkit_experimental::HyperLogLog<'input>,
) -> toolkit_experimental::HyperLogLog<'static> {
    //TODO check type id and collation for equality
    let a = unflatten_log(a);
    let b = unflatten_log(b);
    if a.buildhasher.type_id != b.buildhasher.type_id {
        error!("missmatched types")
    }
    // TODO error on mismatched collation?
    let mut a = a.clone();
    a.merge_in(&b);
    flatten_log(&mut a)
}

fn flatten_log(hyperloglog: &mut HLL<Datum, DatumHashBuilder>)
-> toolkit_experimental::HyperLogLog<'static> {
    let (element_type, collation) = {
        let hasher = &hyperloglog.buildhasher;
        (ShortTypeId(hasher.type_id), PgCollationId(hasher.collation))
    };

    // we need to flatten the vector to a single buffer that contains
    // both the size, the data, and the varlen header

    let flat = match hyperloglog.to_parts() {
        HyperLogLogStorage::Sparse(sparse) => unsafe {
            flatten!(HyperLogLog {
                log: Storage::Sparse {
                    element_type: element_type,
                    collation: collation,
                    num_compressed: sparse.num_compressed,
                    precision: sparse.precision,
                    compressed_bytes: sparse.compressed.num_bytes() as u32,
                    compressed: sparse.compressed.bytes(),
                }
            })
        },
        HyperLogLogStorage::Dense(dense) => unsafe {
            // TODO check that precision and length match?
            flatten!(HyperLogLog {
                log: Storage::Dense {
                    element_type: element_type,
                    collation: collation,
                    precision: dense.precision,
                    registers: dense.registers.bytes(),
                }
            })
        },
    };
    flat.into()
}

fn unflatten_log<'i>(hyperloglog: HyperLogLog<'i>) -> HLL<'i, Datum, DatumHashBuilder> {
    match hyperloglog.log {
        Storage::Sparse {
            num_compressed,
            precision,
            compressed,
            element_type,
            collation,
            compressed_bytes: _
        } => HLL::<Datum, DatumHashBuilder>::from_sparse_parts(
            compressed,
            num_compressed,
            precision,
            unsafe { DatumHashBuilder::from_type_id(element_type.0, Some(collation.0)) }
        ),
        Storage::Dense { precision, registers, element_type, collation  } =>
            HLL::<Datum, DatumHashBuilder>::from_dense_parts(
                registers,
                precision,
                unsafe { DatumHashBuilder::from_type_id(element_type.0, Some(collation.0)) }
            ),
    }
}

// TODO move to it's own mod if we reuse it
struct DatumHashBuilder {
    info: pg_sys::FunctionCallInfo,
    type_id: pg_sys::Oid,
    collation: pg_sys::Oid,
}

impl DatumHashBuilder {
    unsafe fn from_type_id(type_id: pg_sys::Oid, collation: Option<Oid>) -> Self {
        let entry =
            pg_sys::lookup_type_cache(type_id, pg_sys::TYPECACHE_HASH_EXTENDED_PROC_FINFO as _);
        Self::from_type_cache_entry(entry, collation)
    }

    unsafe fn from_type_cache_entry(
        tentry: *const pg_sys::TypeCacheEntry,
        collation: Option<Oid>,
    ) -> Self {
        let flinfo = if (*tentry).hash_extended_proc_finfo.fn_addr.is_some() {
            &(*tentry).hash_extended_proc_finfo
        } else {
            pgx::error!("no hash function");
        };

        // 1 argument for the key, 1 argument for the seed
        let size =
            size_of::<pg_sys::FunctionCallInfoBaseData>() + size_of::<pg_sys::NullableDatum>() * 2;
        let mut info = pg_sys::palloc0(size) as pg_sys::FunctionCallInfo;

        (*info).flinfo = flinfo as *const pg_sys::FmgrInfo as *mut pg_sys::FmgrInfo;
        (*info).context = std::ptr::null_mut();
        (*info).resultinfo = std::ptr::null_mut();
        (*info).fncollation = (*tentry).typcollation;
        (*info).isnull = false;
        (*info).nargs = 1;

        let collation = match collation {
            Some(collation) => collation,
            None => (*tentry).typcollation,
        };

        Self {
            info,
            type_id: (*tentry).type_id,
            collation,
        }
    }
}

impl Clone for DatumHashBuilder {
    fn clone(&self) -> Self {
        Self {
            info: self.info,
            type_id: self.type_id,
            collation: self.collation,
        }
    }
}

impl BuildHasher for DatumHashBuilder {
    type Hasher = DatumHashBuilder;

    fn build_hasher(&self) -> Self::Hasher {
        Self {
            info: self.info,
            type_id: self.type_id,
            collation: self.collation,
        }
    }
}

impl Hasher for DatumHashBuilder {
    fn finish(&self) -> u64 {
        //FIXME ehhh, this is wildly unsafe, should at least have a separate hash
        //      buffer for each, probably should have separate args
        let value = unsafe {
            let value = (*(*self.info).flinfo).fn_addr.unwrap()(self.info);
            (*self.info).args.as_mut_slice(1)[0] = pg_sys::NullableDatum {
                value: 0,
                isnull: true,
            };
            (*self.info).isnull = false;
            //FIXME 32bit vs 64 bit get value from datum on 32b arch
            value
        };
        value as u64
    }

    fn write(&mut self, bytes: &[u8]) {
        if bytes.len() != size_of::<usize>() {
            panic!("invalid datum hash")
        }

        let mut b = [0; size_of::<usize>()];
        for i in 0..size_of::<usize>() {
            b[i] = bytes[i]
        }
        self.write_usize(usize::from_ne_bytes(b))
    }

    fn write_usize(&mut self, i: usize) {
        unsafe {
            (*self.info).args.as_mut_slice(1)[0] = pg_sys::NullableDatum {
                value: i,
                isnull: false,
            };
            (*self.info).isnull = false;
        }
    }
}

impl PartialEq for DatumHashBuilder {
    fn eq(&self, other: &Self) -> bool {
        self.type_id.eq(&other.type_id)
    }
}

impl Eq for DatumHashBuilder {}

impl Serialize for DatumHashBuilder {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let collation = if self.collation == 0 {
            None
        } else {
            Some(PgCollationId(self.collation))
        };
        (ShortTypeId(self.type_id), collation).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DatumHashBuilder {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (type_id, collation) =
            <(ShortTypeId, Option<PgCollationId>)>::deserialize(deserializer)?;
        //FIXME no collation?
        let deserialized = unsafe { Self::from_type_id(type_id.0, collation.map(|c| c.0)) };
        Ok(deserialized)
    }
}

#[cfg(any(test, feature = "pg_test"))]
mod tests {
    use pgx::*;

    #[pg_test]
    fn test_hll_aggregate() {
        Spi::execute(|client| {
            let text = client
                .select(
                    "SELECT \
                        toolkit_experimental.hyperloglog(32, v::float)::TEXT \
                        FROM generate_series(1, 100) v",
                    None,
                    None,
                )
                .first()
                .get_one::<String>();

            let expected = "{\
                \"version\":1,\
                \"log\":{\
                    \"Dense\":{\
                        \"element_type\":\"FLOAT8\",\
                        \"collation\":null,\
                        \"precision\":5,\
                        \"registers\":[\
                            20,64,132,12,81,1,8,64,133,4,64,136,4,82,3,12,17,\
                            65,24,32,197,16,32,132,255\
                        ]\
                    }\
                }\
            }";
            assert_eq!(text.unwrap(), expected);

            let count = client
                .select("SELECT \
                    toolkit_experimental.hyperloglog_count(\
                        toolkit_experimental.hyperloglog(32, v::float)\
                    ) \
                    FROM generate_series(1, 100) v", None, None)
                .first()
                .get_one::<i32>();
            assert_eq!(count, Some(132));

            let count2 = client
                .select(
                    &format!("SELECT toolkit_experimental.hyperloglog_count('{}')", expected),
                    None,
                    None,
                )
                .first()
                .get_one::<i32>();
            assert_eq!(count2, count);
        });
    }

    #[pg_test]
    fn test_hll_aggregate_int() {
        Spi::execute(|client| {
            let text = client
                .select(
                    "SELECT toolkit_experimental.hyperloglog(32, v::int)::TEXT
                    FROM generate_series(1, 100) v",
                    None,
                    None,
                )
                .first()
                .get_one::<String>();

            let expected = "{\
                \"version\":1,\
                \"log\":{\
                    \"Dense\":{\
                        \"element_type\":\"INT4\",\
                        \"collation\":null,\
                        \"precision\":5,\
                        \"registers\":[\
                            8,49,0,12,32,129,24,32,195,16,33,2,12,1,68,4,16,\
                            196,20,64,133,8,17,67,255\
                        ]\
                    }\
                }\
            }";
            assert_eq!(text.unwrap(), expected);

            let count = client
                .select("SELECT \
                toolkit_experimental.hyperloglog_count(\
                    toolkit_experimental.hyperloglog(32, v::int)\
                ) FROM generate_series(1, 100) v", None, None)
                .first()
                .get_one::<i32>();
            assert_eq!(count, Some(96));

            let count2 = client
                .select(
                    &format!("SELECT toolkit_experimental.hyperloglog_count('{}')", expected),
                    None,
                    None,
                )
                .first()
                .get_one::<i32>();
            assert_eq!(count2, count);
        });
    }

    #[pg_test]
    fn test_hll_aggregate_text() {
        Spi::execute(|client| {
            use crate::serialization::PgCollationId;

            let text = client
                .select(
                    "SELECT \
                        toolkit_experimental.hyperloglog(32, v::text)::TEXT \
                    FROM generate_series(1, 100) v",
                    None,
                    None,
                )
                .first()
                .get_one::<String>();

            let default_collation = serde_json::to_string(&PgCollationId(100)).unwrap();
            let expected = format!("{{\
                \"version\":1,\
                \"log\":{{\
                        \"Dense\":{{\
                            \"element_type\":\"TEXT\",\
                            \"collation\":{},\
                            \"precision\":5,\
                            \"registers\":[\
                                12,33,3,8,33,4,20,50,3,12,32,133,4,32,67,8,48,\
                                128,8,33,4,8,32,197,255\
                        ]}}\
                    }}\
                }}", default_collation);
            assert_eq!(text.unwrap(), expected);

            let count = client
                .select("SELECT toolkit_experimental.hyperloglog_count(\
                    toolkit_experimental.hyperloglog(32, v::text)\
                ) FROM generate_series(1, 100) v", None, None)
                .first()
                .get_one::<i32>();
            assert_eq!(count, Some(111));

            let count2 = client
                .select(
                    &format!("SELECT toolkit_experimental.hyperloglog_count('{}')", expected),
                    None,
                    None,
                )
                .first()
                .get_one::<i32>();
            assert_eq!(count2, count);
        });
    }

    #[pg_test]
    fn test_hll_union_text() {
        Spi::execute(|client| {
            {
                // self-union should be a nop
                let expected = client
                    .select(
                        "SELECT \
                                toolkit_experimental.hyperloglog(32, v::text)::TEXT \
                            FROM generate_series(1, 100) v",
                        None,
                        None,
                    )
                    .first()
                    .get_one::<String>()
                    .unwrap();

                let text = client
                    .select(
                        "SELECT toolkit_experimental.rollup(\
                            toolkit_experimental.hyperloglog(32, v::text), \
                            toolkit_experimental.hyperloglog(32, v::text)\
                        )::TEXT FROM generate_series(1, 100) v",
                        None,
                        None,
                    )
                    .first()
                    .get_one::<String>();

                assert_eq!(text.unwrap(), expected);
            }

            {
                // differing unions should be a sum of the distinct counts
                let query =
                    "SELECT toolkit_experimental.hyperloglog_count(\
                        toolkit_experimental.rollup(\
                            (SELECT toolkit_experimental.hyperloglog(32, v::text) FROM generate_series(1, 100) v),\
                            (SELECT toolkit_experimental.hyperloglog(32, v::text) FROM generate_series(50, 150) v)\
                        )\
                    )";
                let count = client
                    .select(
                        query,
                        None,
                        None,
                    )
                    .first()
                    .get_one::<i64>();

                assert_eq!(count, Some(152));
            }

        });
    }

    //TODO test continuous aggregates
}
