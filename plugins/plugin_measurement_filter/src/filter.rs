use tedge_lib::measurement::MeasurementValue;
use tracing::trace;

#[derive(Debug, serde::Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
pub enum Filter {
    #[serde(rename = "is")]
    Is(bool),

    #[serde(rename = "less_than")]
    LessThan(f64),

    #[serde(rename = "more_than")]
    MoreThan(f64),

    #[serde(rename = "contains")]
    Contains(String),

    #[serde(rename = "excludes")]
    Excludes(String),
}

pub trait Filterable {
    fn apply_filter(&self, filter: &Filter) -> bool;
}

impl Filterable for MeasurementValue {
    fn apply_filter(&self, filter: &Filter) -> bool {
        trace!("Filtering with {:?}: {:?}", filter, self);
        match (self, filter) {
            (MeasurementValue::Bool(b1), Filter::Is(b2)) => b1 == b2,
            (MeasurementValue::Bool(_), _) => false,

            (MeasurementValue::Float(f1), Filter::LessThan(f2)) => f1 < f2,
            (MeasurementValue::Float(f1), Filter::MoreThan(f2)) => f1 > f2,
            (MeasurementValue::Float(_), _) => false,

            (MeasurementValue::Text(t1), Filter::Contains(t2)) => t1.contains(t2),
            (MeasurementValue::Text(t1), Filter::Excludes(t2)) => !t1.contains(t2),
            (MeasurementValue::Text(_), _) => false,

            (_, _) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_msmt_bool() {
        let msmt = MeasurementValue::Bool(false);
        let filt = Filter::Is(false);
        assert!(msmt.apply_filter(&filt));

        let filt = Filter::Is(true);
        assert!(!msmt.apply_filter(&filt));
    }

    #[test]
    fn test_filter_msmt_lt() {
        let msmt = MeasurementValue::Float(10.0);
        let filt = Filter::LessThan(20.0);
        assert!(msmt.apply_filter(&filt));

        let filt = Filter::LessThan(5.0);
        assert!(!msmt.apply_filter(&filt));
    }

    #[test]
    fn test_filter_msmt_gt() {
        let msmt = MeasurementValue::Float(10.0);
        let filt = Filter::MoreThan(20.0);
        assert!(!msmt.apply_filter(&filt));

        let filt = Filter::MoreThan(5.0);
        assert!(msmt.apply_filter(&filt));
    }

    #[test]
    fn test_filter_msmt_contains() {
        let msmt = MeasurementValue::Text("foobar".to_string());
        let filt = Filter::Contains("oob".to_string());
        assert!(msmt.apply_filter(&filt));

        let filt = Filter::Contains("kittens".to_string());
        assert!(!msmt.apply_filter(&filt));
    }

    #[test]
    fn test_filter_msmt_excludes() {
        let msmt = MeasurementValue::Text("foobar".to_string());
        let filt = Filter::Excludes("oob".to_string());
        assert!(!msmt.apply_filter(&filt));

        let filt = Filter::Excludes("kittens".to_string());
        assert!(msmt.apply_filter(&filt));
    }

    #[test]
    fn test_filter_nonmatching_bool() {
        let msmt = MeasurementValue::Bool(false);
        let filt = Filter::Excludes("kittens".to_string());
        assert!(!msmt.apply_filter(&filt));
    }

    #[test]
    fn test_filter_nonmatching_float() {
        let msmt = MeasurementValue::Float(1.0);
        let filt = Filter::Excludes("kittens".to_string());
        assert!(!msmt.apply_filter(&filt));
    }

    #[test]
    fn test_filter_nonmatching_string() {
        let msmt = MeasurementValue::Text("foobar".to_string());
        let filt = Filter::Is(true);
        assert!(!msmt.apply_filter(&filt));
    }

}
