#[derive(Debug, serde::Deserialize, tedge_api::Config)]
#[serde_with::serde_as]
pub struct MeasurementFilterConfig {
    /// The name of the plugin to send the measurements to if the filter did not match its value
    pub(crate) target: tedge_lib::config::Address,

    /// The name of the plugin to send measurements to if the filter matched its value
    pub(crate) filtered_target: Option<tedge_lib::config::Address>,

    /// A path to find the value inside a measurement
    #[serde_as(as = "TryFromInto<String>")]
    pub(crate) extractor: crate::extractor::Extractor,

    /// The filter to filter the measurements with
    #[serde(flatten)]
    pub(crate) filter: crate::filter::Filter,
}

#[cfg(test)]
mod tests {
    use super::MeasurementFilterConfig;
    use crate::extractor::Token;
    use crate::filter::Filter;

    #[test]
    fn test_deserialize() {
        let s = r#"
            target = "foo"
            filtered_target = "bar"
            extractor = "foo.bar"
            is = true
        "#;

        let c: MeasurementFilterConfig = toml::from_str(s).unwrap();
        assert_eq!(c.target.as_ref(), "foo");
        assert_eq!(c.filtered_target.as_ref().map(AsRef::as_ref), Some("bar"));
        assert_eq!(
            c.extractor.0,
            vec![Token::Key("foo".to_string()), Token::Key("bar".to_string())]
        );
        assert_eq!(c.filter, Filter::Is(true));
    }
}
