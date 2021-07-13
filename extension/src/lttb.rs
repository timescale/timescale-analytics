
use pgx::*;
use std::borrow::Cow;

use crate::{
    aggregate_utils::in_aggregate_context, flatten, palloc::Internal,
};

use time_series::{TSPoint, TimeSeries as InternalTimeSeries};

use crate::time_series::{TimeSeriesData, SeriesType};

// hack to allow us to qualify names with "toolkit_experimental"
// so that pgx generates the correct SQL
mod toolkit_experimental {
}

pub struct LttbTrans {
    series: InternalTimeSeries,
    resolution: usize,
}

#[pg_extern(schema = "toolkit_experimental")]
pub fn lttb_trans(
    state: Option<Internal<LttbTrans>>,
    time: pg_sys::TimestampTz,
    val: Option<f64>,
    resolution: i32,
    fcinfo: pg_sys::FunctionCallInfo,
) -> Option<Internal<LttbTrans>> {
    unsafe {
        in_aggregate_context(fcinfo, || {
            let val = match val {
                None => return state,
                Some(val) => val,
            };
            let mut state = match state {
                Some(state) => state,
                None => {
                    if resolution <= 2 {
                        error!("resolution must be greater than 2")
                    }
                    LttbTrans {
                        series: InternalTimeSeries::new_explicit_series(),
                        resolution: resolution as usize,
                    }.into()
                },
            };

            state.series.add_point(TSPoint {
                ts: time,
                val: val,
            });
            Some(state)
        })
    }
}

#[pg_extern(schema = "toolkit_experimental")]
pub fn lttb_final(
    state: Option<Internal<LttbTrans>>,
    fcinfo: pg_sys::FunctionCallInfo,
) -> Option<crate::time_series::toolkit_experimental::TimeSeries<'static>> {
    unsafe {
        in_aggregate_context(fcinfo, || {
            let mut state = match state {
                None => return None,
                Some(state) => state,
            };
            state.series.sort();
            let series = Cow::from(&state.series);
            let downsampled = lttb(&*series, state.resolution);
            flatten!(
                TimeSeries {
                    series: SeriesType::SortedSeries {
                        num_points: downsampled.len() as u64,
                        points: &*downsampled,
                    }
                }
            ).into()
        })
    }
}

extension_sql!(r#"
CREATE AGGREGATE toolkit_experimental.lttb(ts TIMESTAMPTZ, value DOUBLE PRECISION, resolution INT) (
    sfunc = toolkit_experimental.lttb_trans,
    stype = internal,
    finalfunc = toolkit_experimental.lttb_final
);
"#);


// based on https://github.com/jeromefroe/lttb-rs version 0.2.0
pub fn lttb(data: &[TSPoint], threshold: usize) -> Cow<'_, [TSPoint]> {
    if threshold >= data.len() || threshold == 0 {
        // Nothing to do.
        return Cow::Borrowed(data)
    }

    let mut sampled = Vec::with_capacity(threshold);

    // Bucket size. Leave room for start and end data points.
    let every = ((data.len() - 2) as f64) / ((threshold - 2) as f64);

    // Initially a is the first point in the triangle.
    let mut a = 0;

    // Always add the first point.
    sampled.push(data[a]);

    for i in 0..threshold - 2 {
        // Calculate point average for next bucket (containing c).
        let mut avg_x = 0i64;
        let mut avg_y = 0f64;

        let avg_range_start = (((i + 1) as f64) * every) as usize + 1;

        let mut end = (((i + 2) as f64) * every) as usize + 1;
        if end >= data.len() {
            end = data.len();
        }
        let avg_range_end = end;

        let avg_range_length = (avg_range_end - avg_range_start) as f64;

        for i in 0..(avg_range_end - avg_range_start) {
            let idx = (avg_range_start + i) as usize;
            avg_x += data[idx].ts;
            avg_y += data[idx].val;
        }
        avg_x /= avg_range_length as i64;
        avg_y /= avg_range_length;

        // Get the range for this bucket.
        let range_offs = ((i as f64) * every) as usize + 1;
        let range_to = (((i + 1) as f64) * every) as usize + 1;

        // Point a.
        let point_a_x = data[a].ts;
        let point_a_y = data[a].val;

        let mut max_area = -1f64;
        let mut next_a = range_offs;
        for i in 0..(range_to - range_offs) {
            let idx = (range_offs + i) as usize;

            // Calculate triangle area over three buckets.
            let area = ((point_a_x - avg_x) as f64 * (data[idx].val - point_a_y)
                - (point_a_x - data[idx].ts) as f64 * (avg_y - point_a_y))
                .abs()
                * 0.5;
            if area > max_area {
                max_area = area;
                next_a = idx; // Next a is this b.
            }
        }

        sampled.push(data[next_a]); // Pick this point from the bucket.
        a = next_a; // This a is the next a (chosen b).
    }

    // Always add the last point.
    sampled.push(data[data.len() - 1]);

    Cow::Owned(sampled)
}

#[pg_extern(name="lttb", schema = "toolkit_experimental")]
pub fn lttb_on_timeseries(
    series: crate::time_series::toolkit_experimental::TimeSeries<'static>,
    threshold: i32,
) -> Option<crate::time_series::toolkit_experimental::TimeSeries<'static>> {
    lttb_ts(series, threshold as usize).into()
}

// based on https://github.com/jeromefroe/lttb-rs version 0.2.0
pub fn lttb_ts<'s>(
    data: crate::time_series::toolkit_experimental::TimeSeries<'s>,
    threshold: usize
)
-> crate::time_series::toolkit_experimental::TimeSeries<'static>
{
    if !data.is_sorted() {
        panic!("lttb requires sorted timeseries");
    }

    if threshold >= data.num_points() || threshold == 0 {
        // Nothing to do.
        return data.in_current_context();  // can we avoid this copy???
    }

    // let mut sampled = Vec::with_capacity(threshold);
    let mut sampled = InternalTimeSeries::new_explicit_series();

    // Bucket size. Leave room for start and end data points.
    let every = ((data.num_points() - 2) as f64) / ((threshold - 2) as f64);

    // Initially a is the first point in the triangle.
    let mut a = 0;

    // Always add the first point.
    sampled.add_point(data.get(a).unwrap());

    for i in 0..threshold - 2 {
        // Calculate point average for next bucket (containing c).
        let mut avg_x = 0i64;
        let mut avg_y = 0f64;

        let avg_range_start = (((i + 1) as f64) * every) as usize + 1;

        let mut end = (((i + 2) as f64) * every) as usize + 1;
        if end >= data.num_points() {
            end = data.num_points();
        }
        let avg_range_end = end;

        let avg_range_length = (avg_range_end - avg_range_start) as f64;

        for i in 0..(avg_range_end - avg_range_start) {
            let idx = (avg_range_start + i) as usize;
            let point = data.get(idx).unwrap();
            avg_x += point.ts;
            avg_y += point.val;
        }
        avg_x /= avg_range_length as i64;
        avg_y /= avg_range_length;

        // Get the range for this bucket.
        let range_offs = ((i as f64) * every) as usize + 1;
        let range_to = (((i + 1) as f64) * every) as usize + 1;

        // Point a.
        let point_a_x = data.get(a).unwrap().ts;
        let point_a_y = data.get(a).unwrap().val;

        let mut max_area = -1f64;
        let mut next_a = range_offs;
        for i in 0..(range_to - range_offs) {
            let idx = (range_offs + i) as usize;

            // Calculate triangle area over three buckets.
            let area = ((point_a_x - avg_x) as f64 * (data.get(idx).unwrap().val - point_a_y)
                - (point_a_x - data.get(idx).unwrap().ts) as f64 * (avg_y - point_a_y))
                .abs()
                * 0.5;
            if area > max_area {
                max_area = area;
                next_a = idx; // Next a is this b.
            }
        }

        sampled.add_point(data.get(next_a).unwrap()); // Pick this point from the bucket.
        a = next_a; // This a is the next a (chosen b).
    }

    // Always add the last point.
    sampled.add_point(data.get(data.num_points() - 1).unwrap());

    crate::time_series::toolkit_experimental::TimeSeries::from_internal_time_series(&sampled)
}

#[cfg(any(test, feature = "pg_test"))]
mod tests {
    use pgx::*;

    #[pg_test]
    fn test_lttb_equivalence() {
        Spi::execute(|client| {
            client.select("CREATE TABLE test(time TIMESTAMPTZ, value DOUBLE PRECISION);", None, None);
            client.select(
                "INSERT INTO test
                SELECT time, value
                FROM toolkit_experimental.generate_periodic_normal_series('2020-01-01 UTC'::timestamptz, NULL);", None, None);

            client.select("CREATE TABLE results1(time TIMESTAMPTZ, value DOUBLE PRECISION);", None, None);
            client.select(
                "INSERT INTO results1
                SELECT time, value
                FROM toolkit_experimental.unnest_series(
                    (SELECT toolkit_experimental.lttb(time, value, 100) FROM test)
                );", None, None);

            client.select("CREATE TABLE results2(time TIMESTAMPTZ, value DOUBLE PRECISION);", None, None);
            client.select(
                "INSERT INTO results2
                SELECT time, value
                FROM toolkit_experimental.unnest_series(
                    (SELECT toolkit_experimental.lttb(
                        (SELECT toolkit_experimental.timeseries(time, value) FROM test), 100)
                    )
                );", None, None);

            let delta = client
                .select("SELECT count(*)  FROM results1 r1 FULL OUTER JOIN results2 r2 ON r1 = r2 WHERE r1 IS NULL OR r2 IS NULL;" , None, None)
                .first()
                .get_one::<i32>();
            assert_eq!(delta.unwrap(), 0);
        })
    }
}
