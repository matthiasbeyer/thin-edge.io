#[derive(Debug, serde::Deserialize)]
#[serde_with::serde_as]
pub struct MeasurementFilterConfig {
    pub(crate) target: String,
    pub(crate) filtered_target: Option<String>,

    #[serde_as(as = "TryFromInto<String>")]
    pub(crate) extractor: crate::extractor::Extractor,

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
        assert_eq!(c.target, "foo");
        assert_eq!(c.filtered_target, Some("bar".to_string()));
        assert_eq!(c.extractor.0, vec![Token::Key("foo".to_string()), Token::Key("bar".to_string())]);
        assert_eq!(c.filter, Filter::Is(true));
    }
}

