#[derive(Copy, Clone, Debug, serde::Deserialize)]
#[serde(transparent)]
pub struct Humantime(#[serde(with = "humantime_serde")] std::time::Duration);

impl Humantime {
    pub fn into_duration(self) -> std::time::Duration {
        self.0
    }
}

impl tedge_api::AsConfig for Humantime {
    fn as_config() -> tedge_api::ConfigDescription {
        tedge_api::ConfigDescription::new(
            "Duration-representing String".to_string(),
            tedge_api::ConfigKind::String,
            Some(indoc::indoc! {r#"
                A String that represents a duration

                ## Examples

                A duration of one minute:

                ```toml
                "1min"
                ```

                A duration of 5 minutes and 2 nanoseconds:

                ```toml
                "5min 2ns"
                ```

                ## More information

                For more information have a look at the documentation of
                [the humantime crate](https://docs.rs/humantime).
            "#}),
        )
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_humantime_deser() {
        let ts = r#"t = "1sec""#;

        #[derive(serde::Deserialize)]
        struct T {
            t: super::Humantime,
        }

        let ht: T = toml::from_str(ts).unwrap();
        assert_eq!(ht.t.0, std::time::Duration::from_secs(1));
    }
}
