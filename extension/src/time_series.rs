
use std::{slice};

use pgx::*;

use crate::{
    aggregate_utils::in_aggregate_context, pg_type, flatten, palloc::Internal,
};

use time_series::{TSPoint, TimeSeries as InternalTimeSeries, ExplicitTimeSeries, NormalTimeSeries, GapfillMethod};

use flat_serialize::*;

mod pipeline;

#[allow(non_camel_case_types)]
type bytea = pg_sys::Datum;

pg_type! {
    #[derive(Debug, Copy)]
    struct TimeSeries<'input> {
        series: enum SeriesType<'input> {
            type_id: u64,
            SortedSeries: 1 {
                num_points: u64,  // required to be aligned
                points: [TSPoint; self.num_points],
            },
            NormalSeries: 2 {
                start_ts: i64,
                step_interval: i64,
                num_vals: u64,  // required to be aligned
                values: [f64; self.num_vals],
            },
            // ExplicitSeries is assumed to be unordered
            ExplicitSeries: 3 {
                num_points: u64,  // required to be aligned
                points: [TSPoint; self.num_points],
            },
        },
    }
}

impl<'input> InOutFuncs for TimeSeries<'input> {
    fn output(&self, buffer: &mut StringInfo) {
        use crate::serialization::{EncodedStr::*, str_to_db_encoding};

        // TODO remove extra allocation
        // FIXME print timestamps as times, not integers
        let serializer: Vec<_> = self.to_internal_time_series()
            .iter()
            // .map(|point| (point.ts, point.val))
            .collect();

        let stringified = serde_json::to_string(&*serializer).unwrap();
        match str_to_db_encoding(&stringified) {
            Utf8(s) => buffer.push_str(s),
            Other(s) => buffer.push_bytes(s.to_bytes()),
        }
    }

    fn input(input: &std::ffi::CStr) -> Self
    where
        Self: Sized,
    {
        use crate::serialization::str_from_db_encoding;

        // SAFETY our serde shims will allocate and leak copies of all
        // the data, so the lifetimes of the borrows aren't actually
        // relevant to the output lifetime
        // TODO reduce allocation
        let series: Vec<TSPoint> = unsafe {
            unsafe fn extend_lifetime(s: &str) -> &'static str {
                std::mem::transmute(s)
            }
            let input = extend_lifetime(str_from_db_encoding(input));
            serde_json::from_str(input).unwrap()
        };
        unsafe {
            flatten! {
                TimeSeries {
                    series: SeriesType::ExplicitSeries {
                        num_points: series.len() as u64,
                        points: &*series,
                    }
                }
            }
        }
    }
}

// hack to allow us to qualify names with "toolkit_experimental"
// so that pgx generates the correct SQL
pub mod toolkit_experimental {
    pub(crate) use super::*;
    varlena_type!(TimeSeries);
}

impl<'input> TimeSeries<'input> {
    #[allow(dead_code)]
    pub fn to_internal_time_series(&self) -> InternalTimeSeries {
        match self.series {
            SeriesType::SortedSeries{points, ..} =>
                InternalTimeSeries::Explicit(
                    ExplicitTimeSeries {
                        ordered: true,
                        points: points.to_vec(),
                    }
                ),
            // This is assumed unordered
            SeriesType::ExplicitSeries{points, ..} =>
                    InternalTimeSeries::Explicit(
                        ExplicitTimeSeries {
                            ordered: false,
                            points: points.to_vec(),
                        }
                    ),
            SeriesType::NormalSeries{start_ts, step_interval, values, ..} =>
                InternalTimeSeries::Normal(
                    NormalTimeSeries {
                        start_ts: start_ts,
                        step_interval: step_interval,
                        values: values.to_vec(),
                    }
                ),
        }
    }

    pub fn num_points(&self) -> usize {
        match self.series {
            SeriesType::SortedSeries{points, ..} =>
                points.len(),
            SeriesType::ExplicitSeries{points, ..} =>
                points.len(),
            SeriesType::NormalSeries{values, ..} =>
                values.len(),
        }
    }

    pub fn from_internal_time_series(series: &InternalTimeSeries) -> TimeSeries<'static> {
        unsafe {
            match series {
                InternalTimeSeries::Explicit(series) => {
                    if !series.ordered {
                        flatten!(
                            TimeSeries {
                                series: SeriesType::ExplicitSeries {
                                    num_points: series.points.len() as u64,
                                    points: &series.points,
                                }
                            }
                        )
                    } else {
                        flatten!(
                            TimeSeries {
                                series: SeriesType::SortedSeries {
                                    num_points: series.points.len() as u64,
                                    points: &series.points,
                                }
                            }
                        )
                    }
                },
                InternalTimeSeries::Normal(series) => {
                    flatten!(
                        TimeSeries {
                            series : SeriesType::NormalSeries {
                                start_ts: series.start_ts,
                                step_interval: series.step_interval,
                                num_vals: series.values.len() as u64,
                                values: &series.values,
                            }
                        }
                    )
                }
            }
        }
    }

    // Gets the nth point of a timeseries
    // Differs from normal vector get in that it returns a copy rather than a reference (as the point may have to be constructed)
    pub fn get(&self, index: usize) -> Option<TSPoint> {
        if index >= self.num_points() {
            return None;
        }

        match self.series {
            SeriesType::SortedSeries{points, ..} =>
                Some(points[index]),
            SeriesType::ExplicitSeries{points, ..} =>
                Some(points[index]),
            SeriesType::NormalSeries{start_ts, step_interval, values, ..} =>
                Some(TSPoint{ts: start_ts + index as i64 * step_interval, val: values[index]}),
        }
    }

    pub fn is_sorted(&self) -> bool {
        match self.series {
            SeriesType::SortedSeries{..} =>
                true,
            SeriesType::ExplicitSeries{..} =>
                false, // a sorted ExplicitSeries is written out as a SortedSeries
            SeriesType::NormalSeries{..} =>
                true,
        }
    }
}

#[pg_extern(schema = "toolkit_experimental")]
pub fn unnest_series(
    series: toolkit_experimental::TimeSeries,
) -> impl std::iter::Iterator<Item = (name!(time,pg_sys::TimestampTz),name!(value,f64))> + '_ {
    let iter: Box<dyn Iterator<Item=_>> = match series.series {
        SeriesType::SortedSeries{points, ..} =>
            Box::new(points.iter().map(|points| (points.ts, points.val))),

        SeriesType::ExplicitSeries{points, ..} =>
            Box::new(points.iter().map(|points| (points.ts, points.val))),

        SeriesType::NormalSeries{start_ts, step_interval, num_vals, values} =>
            Box::new((0..num_vals).map(move |i| {
                let num_steps = i as i64;
                let step_interval = step_interval;
                (start_ts + num_steps * step_interval, values[i as usize])
            })),
    };
    iter
}

#[pg_extern(schema = "toolkit_experimental")]
pub fn timeseries_serialize(
    state: Internal<InternalTimeSeries>,
) -> bytea {
    crate::do_serialize!(state)
}

#[pg_extern(schema = "toolkit_experimental",strict)]
pub fn timeseries_deserialize(
    bytes: bytea,
    _internal: Option<Internal<()>>,
) -> Internal<InternalTimeSeries> {
    crate::do_deserialize!(bytes, InternalTimeSeries)
}

#[pg_extern(schema = "toolkit_experimental")]
pub fn timeseries_trans(
    state: Option<Internal<InternalTimeSeries>>,
    time: Option<pg_sys::TimestampTz>,
    value: Option<f64>,
    fcinfo: pg_sys::FunctionCallInfo,
) -> Option<Internal<InternalTimeSeries>> {
    unsafe {
        in_aggregate_context(fcinfo, || {
            let time = match time {
                None => return state,
                Some(time) => time,
            };
            let value = match value {
                None => return state,   // Should we support NULL values?
                Some(value) => value,
            };
            let mut state = match state {
                None => InternalTimeSeries::new_explicit_series().into(),
                Some(state) => state,
            };
            state.add_point(TSPoint{ts: time, val:value});
            Some(state)
        })
    }
}

#[pg_extern(schema = "toolkit_experimental")]
pub fn timeseries_compound_trans(
    state: Option<Internal<InternalTimeSeries>>,
    series: Option<crate::time_series::toolkit_experimental::TimeSeries<'static>>,
    fcinfo: pg_sys::FunctionCallInfo,
) -> Option<Internal<InternalTimeSeries>> {
    unsafe {
        in_aggregate_context(fcinfo, || {
            match (state, series) {
                (None, None) => None,
                (None, Some(series)) => Some(series.to_internal_time_series().into()),
                (Some(state), None) => Some(state.clone().into()),
                (Some(state), Some(series)) =>
                    Some(InternalTimeSeries::combine(&state, &series.to_internal_time_series()).into())
            }
        })
    }
}

#[pg_extern(schema = "toolkit_experimental")]
pub fn timeseries_combine (
    state1: Option<Internal<InternalTimeSeries>>,
    state2: Option<Internal<InternalTimeSeries>>,
    fcinfo: pg_sys::FunctionCallInfo,
) -> Option<Internal<InternalTimeSeries>> {
    unsafe {
        in_aggregate_context(fcinfo, || {
            match (state1, state2) {
                (None, None) => None,
                (None, Some(state2)) => Some(state2.clone().into()),
                (Some(state1), None) => Some(state1.clone().into()),
                (Some(state1), Some(state2)) =>
                    Some(InternalTimeSeries::combine(&state1, &state2).into())
            }
        })
    }
}

#[pg_extern(schema = "toolkit_experimental")]
pub fn timeseries_final(
    state: Option<Internal<InternalTimeSeries>>,
    fcinfo: pg_sys::FunctionCallInfo,
) -> Option<crate::time_series::toolkit_experimental::TimeSeries<'static>> {
    unsafe {
        in_aggregate_context(fcinfo, || {
            let state = match state {
                None => return None,
                Some(state) => state,
            };
            TimeSeries::from_internal_time_series(&state).into()
        })
    }
}

extension_sql!(r#"
CREATE AGGREGATE toolkit_experimental.timeseries(ts TIMESTAMPTZ, value DOUBLE PRECISION) (
    sfunc = toolkit_experimental.timeseries_trans,
    stype = internal,
    finalfunc = toolkit_experimental.timeseries_final,
    combinefunc = toolkit_experimental.timeseries_combine,
    serialfunc = toolkit_experimental.timeseries_serialize,
    deserialfunc = toolkit_experimental.timeseries_deserialize
);
"#);

extension_sql!(r#"
CREATE AGGREGATE toolkit_experimental.rollup(
    toolkit_experimental.timeseries
) (
    sfunc = toolkit_experimental.timeseries_compound_trans,
    stype = internal,
    finalfunc = toolkit_experimental.timeseries_final,
    combinefunc = toolkit_experimental.timeseries_combine,
    serialfunc = toolkit_experimental.timeseries_serialize,
    deserialfunc = toolkit_experimental.timeseries_deserialize
);
"#);

type Interval = pg_sys::Datum;

#[pg_extern(schema = "toolkit_experimental", name="normalize")]
pub fn normalize_default_range (
    series: crate::time_series::toolkit_experimental::TimeSeries<'static>,
    interval: Interval,
    method: String,
    truncate: Option<bool>,
    _fcinfo: pg_sys::FunctionCallInfo,
) -> Option<crate::time_series::toolkit_experimental::TimeSeries<'static>> {
    normalize(series, interval, method, truncate, None, None, _fcinfo)
}

#[pg_extern(schema = "toolkit_experimental")]
pub fn normalize (
    series: crate::time_series::toolkit_experimental::TimeSeries<'static>,
    interval: Interval,
    method: String,
    truncate: Option<bool>,
    range_start: Option<pg_sys::TimestampTz>,
    range_end: Option<pg_sys::TimestampTz>,
    _fcinfo: pg_sys::FunctionCallInfo,
) -> Option<crate::time_series::toolkit_experimental::TimeSeries<'static>> {
    unsafe {
        let interval = interval as *const pg_sys::Interval;
        if (*interval).day > 0 || (*interval).month > 0 {
            panic!("Normalization intervals are currently restricted to stable units (hours or smaller)");
        }
        let interval = (*interval).time;
        let method = match method.to_ascii_lowercase().as_str() {
            "locf" => GapfillMethod::LOCF,
            "nearest" => GapfillMethod::Nearest,
            "interpolate" => GapfillMethod::Linear,
            _ => panic!("Unknown normalization method: {} - valid methods are locf, nearest, or interpolate", method)
        };
        let truncate = match truncate {
            Some(x) => x,
            None => true,
        };
        if series.len() < 2 {
            panic!("Need at least two points to normalize a timeseries")
        }

        // TODO: if series is sorted we should be able to do this without a copy
        let mut series = series.to_internal_time_series();
        series.sort();

        let align = if truncate {interval} else {1};
        let start = match range_start {
            Some(t) => t,
            None => series.first().unwrap().ts,
        } / align * align;

        let end = match range_end {
            Some(t) => t,
            None => series.last().unwrap().ts,
        } / align * align;

        let mut iter = series.iter().peekable();
        let mut first = iter.next().unwrap();
        let mut second = iter.next().unwrap();

        while second.ts < start && iter.peek().is_some() {
            first = second;
            second = iter.next().unwrap();
        }


        // TODO: should be able to create new TimeSeries in place
        let mut result =
            InternalTimeSeries::new_normal_series(
                if start < first.ts {
                    method.predict_left(start, first, Some(second))
                } else if start == first.ts {
                    first
                } else if start < second.ts {
                    method.gapfill(start, first, second)
                } else {
                    method.predict_right(start, second, Some(first))
                }, interval);

        let mut next = start + interval;

        while next < first.ts {
            result.add_point(method.predict_left(next, first, Some(second)));
            next += interval;
        }

        let mut left = first;
        let mut right = second;

        while next <= end {
            if next == left.ts {
                result.add_point(left);
                next += interval;
            }
            while next < right.ts && next <= end {
                result.add_point(method.gapfill(next, left, right));
                next += interval;
            }
            if iter.peek().is_some() {
                left = right;
                right = iter.next().unwrap();
            } else {
                while next <= end {
                    // This will still behave correctly if next == right.ts
                    result.add_point(method.predict_right(next, right, Some(left)));
                    next += interval;
                }
            }
        }

        Some(TimeSeries::from_internal_time_series(&result))
    }
}


#[cfg(any(test, feature = "pg_test"))]
mod tests {
    use pgx::*;

    #[pg_test]
    fn test_normalization_gapfill() {
        Spi::execute(|client| {
            client.select("CREATE TABLE test(time TIMESTAMPTZ, value DOUBLE PRECISION);", None, None);
            client.select(
                "INSERT INTO test
                SELECT '2020-01-01 0:02 UTC'::timestamptz + '10 minutes'::interval * i, 10.0 * i
                FROM generate_series(0,6) as i", None, None);

            client.select("set timescaledb_toolkit_acknowledge_auto_drop to 'true'", None, None);
            client.select("CREATE VIEW series AS SELECT toolkit_experimental.timeseries(time, value) FROM test;", None, None);

            // LOCF
            let results = client.select(
                "SELECT value
                FROM toolkit_experimental.unnest_series(
                    (SELECT toolkit_experimental.normalize(timeseries, '10 min', 'locf', true)
                     FROM series)
                );", None, None);

            let expected = vec![0.0, 0.0, 10.0, 20.0, 30.0, 40.0, 50.0];

            assert_eq!(results.len(), expected.len());

            for (e, r) in expected.iter().zip(results) {
                assert_eq!(r.by_ordinal(1).unwrap().value::<f64>().unwrap(), *e);
            }

            // Interpolate
            let results = client.select(
                "SELECT value
                FROM toolkit_experimental.unnest_series(
                    (SELECT toolkit_experimental.normalize(timeseries, '10 min', 'interpolate', true)
                     FROM series)
                );", None, None);

            let expected = vec![-2.0, 8.0, 18.0, 28.0, 38.0, 48.0, 58.0];

            assert_eq!(results.len(), expected.len());

            for (e, r) in expected.iter().zip(results) {
                assert_eq!(r.by_ordinal(1).unwrap().value::<f64>().unwrap(), *e);
            }

            // Nearest
            let results = client.select(
                "SELECT value
                FROM toolkit_experimental.unnest_series(
                    (SELECT toolkit_experimental.normalize(timeseries, '10 min', 'nearest', true)
                     FROM series)
                );", None, None);

            let expected = vec![0.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0];

            assert_eq!(results.len(), expected.len());

            for (e, r) in expected.iter().zip(results) {
                assert_eq!(r.by_ordinal(1).unwrap().value::<f64>().unwrap(), *e);
            }

        })
    }
}
