
use std::convert::TryInto;

use pgx::*;

use super::*;

use crate::{
    json_inout_funcs, pg_type, flatten,
};

// TODO once we start stabilizing elements, create a type
//      `TimeseriesPipelineElement` and move stable variants to that.
pg_type! {
    #[derive(Debug)]
    struct UnstableTimeseriesPipelineElement {
        element: enum Element {
            kind: u64,
            LTTB: 1 {
                resolution: u64,
            },
        },
    }
}

json_inout_funcs!(UnstableTimeseriesPipelineElement);

// TODO once we start stabilizing elements, create a type TimeseriesPipeline
//      stable elements will create a stable pipeline, but adding an unstable
//      element to a stable pipeline will create an unstable pipeline
type USPED = UnstableTimeseriesPipelineElementData;
pg_type! {
    #[derive(Debug)]
    struct UnstableTimeseriesPipeline<'input> {
        num_elements: u64,
        elements: [USPED; self.num_elements],
    }
}

json_inout_funcs!(UnstableTimeseriesPipeline);

// hack to allow us to qualify names with "toolkit_experimental"
// so that pgx generates the correct SQL
pub mod toolkit_experimental {
    pub(crate) use super::*;
    varlena_type!(UnstableTimeseriesPipeline);
    varlena_type!(UnstableTimeseriesPipelineElement);
}

pub enum MaybeOwnedTs<'s> {
    Borrowed(TimeSeries<'s>),
    Owned(TimeSeries<'static>),
}

#[pg_extern(immutable, parallel_safe, schema="toolkit_experimental")]
pub fn run_pipeline<'s, 'p>(
    timeseries: toolkit_experimental::TimeSeries<'s>,
    pipeline: toolkit_experimental::UnstableTimeseriesPipeline<'p>,
) -> toolkit_experimental::TimeSeries<'static> {
    let mut timeseries = MaybeOwnedTs::Borrowed(timeseries);
    for element in pipeline.elements.iter() {
        let element = element.element;
        let new_timeseries = execute_pipeline_element(&mut timeseries, &element);
        if let Some(series) = new_timeseries {
            timeseries = MaybeOwnedTs::Owned(series)
        }
    }
    match timeseries {
        MaybeOwnedTs::Borrowed(series) => series.in_current_context(),
        MaybeOwnedTs::Owned(series) => series,
    }
}

#[pg_extern(immutable, parallel_safe, schema="toolkit_experimental")]
pub fn run_pipeline_element<'s, 'p>(
    timeseries: toolkit_experimental::TimeSeries<'s>,
    element: toolkit_experimental::UnstableTimeseriesPipelineElement<'p>,
) -> toolkit_experimental::TimeSeries<'static> {
    let owned_timeseries = execute_pipeline_element(&mut MaybeOwnedTs::Borrowed(timeseries), &element.element);
    if let Some(timeseries) = owned_timeseries {
        return timeseries
    }
    return timeseries.in_current_context()
}

// TODO need cow-like for timeseries input
pub fn execute_pipeline_element<'s, 'e>(
    timeseries: &mut MaybeOwnedTs<'s>,
    element: &Element
) -> Option<toolkit_experimental::TimeSeries<'static>> {
    use MaybeOwnedTs::{Borrowed, Owned};
    match (element, timeseries) {
        (Element::LTTB{resolution}, Borrowed(timeseries)) => {
            return Some(crate::lttb::lttb_ts(*timeseries, *resolution as _))
        }
        (Element::LTTB{resolution}, Owned(timeseries)) => {
            return Some(crate::lttb::lttb_ts(*timeseries, *resolution as _))
        }
    }
}


// TODO is (immutable, parallel_safe) correct?
#[pg_extern(immutable, parallel_safe, schema="toolkit_experimental")]
pub fn add_unstable_element<'p, 'e>(
    pipeline: toolkit_experimental::UnstableTimeseriesPipeline<'p>,
    element: toolkit_experimental::UnstableTimeseriesPipelineElement<'e>,
) -> toolkit_experimental::UnstableTimeseriesPipeline<'p> {
    unsafe {
        let elements: Vec<_> = pipeline.elements.iter().chain(Some(element.flatten().0)).collect();
        flatten! {
            UnstableTimeseriesPipeline {
                num_elements: elements.len().try_into().unwrap(),
                elements: (&*elements).into(),
            }
        }
    }
}

// using this instead of pg_operator since the latter doesn't support schemas yet
// FIXME there is no CREATE OR REPLACE OPERATOR need to update post-install.rs
//       need to ensure this works with out unstable warning
extension_sql!(r#"
CREATE OPERATOR toolkit_experimental.|> (
    PROCEDURE=toolkit_experimental."run_pipeline",
    LEFTARG=toolkit_experimental.TimeSeries,
    RIGHTARG=toolkit_experimental.UnstableTimeseriesPipeline
);

CREATE OPERATOR toolkit_experimental.|> (
    PROCEDURE=toolkit_experimental."run_pipeline_element",
    LEFTARG=toolkit_experimental.TimeSeries,
    RIGHTARG=toolkit_experimental.UnstableTimeseriesPipelineElement
);
"#);

// TODO is (immutable, parallel_safe) correct?
#[pg_extern(
    immutable,
    parallel_safe,
    name="lttb",
    schema="toolkit_experimental"
)]
pub fn lttb_pipeline_element<'p, 'e>(
    resolution: i32,
) -> toolkit_experimental::UnstableTimeseriesPipelineElement<'e> {
    unsafe {
        flatten!(
            UnstableTimeseriesPipelineElement {
                element: Element::LTTB {
                    resolution: resolution.try_into().unwrap(),
                }
            }
        )
    }
}

#[cfg(any(test, feature = "pg_test"))]
mod tests {
    use pgx::*;

    #[pg_test]
    fn test_pipeline_lttb() {
        Spi::execute(|client| {
            // using the search path trick for this test b/c the operator is
            // difficult to spot otherwise.
            let sp = client.select("SELECT format(' %s, toolkit_experimental',current_setting('search_path'))", None, None).first().get_one::<String>().unwrap();
            client.select(&format!("SET LOCAL search_path TO {}", sp), None, None);
            client.select("SET timescaledb_toolkit_acknowledge_auto_drop TO 'true'", None, None);

            client.select(
                "CREATE TABLE lttb_pipe (series timeseries)",
                None,
                None
            );
            client.select(
                "INSERT INTO lttb_pipe \
                SELECT timeseries(time, val) FROM ( \
                    SELECT \
                        '2020-01-01 UTC'::TIMESTAMPTZ + make_interval(days=>(foo*10)::int) as time, \
                        10 + 5 * cos(foo) as val \
                    FROM generate_series(1,11,0.1) foo \
                ) bar",
                None,
                None
            );

            let val = client.select(
                "SELECT (series |> lttb(17))::TEXT FROM lttb_pipe",
                None,
                None
            )
                .first()
                .get_one::<String>();
            assert_eq!(val.unwrap(), "[\
                {\"ts\":\"2020-01-11 00:00:00+00\",\"val\":12.7015115293407},\
                {\"ts\":\"2020-01-13 00:00:00+00\",\"val\":11.811788772383368},\
                {\"ts\":\"2020-01-22 00:00:00+00\",\"val\":7.475769477000712},\
                {\"ts\":\"2020-01-28 00:00:00+00\",\"val\":5.479639289914694},\
                {\"ts\":\"2020-02-03 00:00:00+00\",\"val\":5.062601150455675},\
                {\"ts\":\"2020-02-09 00:00:00+00\",\"val\":6.370338478999299},\
                {\"ts\":\"2020-02-14 00:00:00+00\",\"val\":8.463335650107904},\
                {\"ts\":\"2020-02-24 00:00:00+00\",\"val\":13.173464379713174},\
                {\"ts\":\"2020-03-01 00:00:00+00\",\"val\":14.80085143325183},\
                {\"ts\":\"2020-03-07 00:00:00+00\",\"val\":14.751162959792648},\
                {\"ts\":\"2020-03-13 00:00:00+00\",\"val\":13.041756572661273},\
                {\"ts\":\"2020-03-23 00:00:00+00\",\"val\":8.304225695080827},\
                {\"ts\":\"2020-03-29 00:00:00+00\",\"val\":5.94453492969172},\
                {\"ts\":\"2020-04-04 00:00:00+00\",\"val\":5.0015347898239675},\
                {\"ts\":\"2020-04-10 00:00:00+00\",\"val\":5.804642354617738},\
                {\"ts\":\"2020-04-14 00:00:00+00\",\"val\":7.195078712863856},\
                {\"ts\":\"2020-04-20 00:00:00+00\",\"val\":10.022128489940254}\
            ]");

            let val = client.select(
                "SELECT (series |> lttb(8))::TEXT FROM lttb_pipe",
                None,
                None
            )
                .first()
                .get_one::<String>();
            assert_eq!(val.unwrap(), "[\
                {\"ts\":\"2020-01-11 00:00:00+00\",\"val\":12.7015115293407},\
                {\"ts\":\"2020-01-27 00:00:00+00\",\"val\":5.715556233155263},\
                {\"ts\":\"2020-02-06 00:00:00+00\",\"val\":5.516207918329265},\
                {\"ts\":\"2020-02-27 00:00:00+00\",\"val\":14.173563924195799},\
                {\"ts\":\"2020-03-09 00:00:00+00\",\"val\":14.346987451749126},\
                {\"ts\":\"2020-03-30 00:00:00+00\",\"val\":5.672823953794438},\
                {\"ts\":\"2020-04-09 00:00:00+00\",\"val\":5.554044236873196},\
                {\"ts\":\"2020-04-20 00:00:00+00\",\"val\":10.022128489940254}\
            ]");

            let val = client.select(
                "SELECT (series |> lttb(8) |> lttb(8))::TEXT FROM lttb_pipe",
                None,
                None
            )
                .first()
                .get_one::<String>();
            assert_eq!(val.unwrap(), "[\
                {\"ts\":\"2020-01-11 00:00:00+00\",\"val\":12.7015115293407},\
                {\"ts\":\"2020-01-27 00:00:00+00\",\"val\":5.715556233155263},\
                {\"ts\":\"2020-02-06 00:00:00+00\",\"val\":5.516207918329265},\
                {\"ts\":\"2020-02-27 00:00:00+00\",\"val\":14.173563924195799},\
                {\"ts\":\"2020-03-09 00:00:00+00\",\"val\":14.346987451749126},\
                {\"ts\":\"2020-03-30 00:00:00+00\",\"val\":5.672823953794438},\
                {\"ts\":\"2020-04-09 00:00:00+00\",\"val\":5.554044236873196},\
                {\"ts\":\"2020-04-20 00:00:00+00\",\"val\":10.022128489940254}\
            ]");
        });
    }
}