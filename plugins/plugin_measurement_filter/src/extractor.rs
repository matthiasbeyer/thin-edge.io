use tracing::trace;

#[derive(Debug, serde_with::DeserializeFromStr)]
pub struct Extractor(pub(crate) Vec<Token>);

impl From<Extractor> for String {
    fn from(e: Extractor) -> String {
        e.0.iter().map(Token::to_string).collect()
    }
}

impl std::str::FromStr for Extractor {
    type Err = String;

    fn from_str(s: &str) -> Result<Extractor, Self::Err> {
        // TODO: Make this implementation bullet-proof with nom or something like that
        let v = s.split('.')
            .map(Token::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        if v.is_empty() || v.get(0).map(|t| t.is_empty()).unwrap_or(false) {
            return Err("Empty extractor".to_string())
        }

        Ok(Extractor(v))
    }
}

#[derive(Debug, serde::Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
pub enum Token {
    Key(String),
    Index(usize),
}

impl Token {
    /// Helper for checking whether a Token::Key is empty
    fn is_empty(&self) -> bool {
        match self {
            Token::Key(s) => s.is_empty(),
            _ => false,
        }
    }
}

impl ToString for Token {
    fn to_string(&self) -> String {
        match self {
            Token::Key(s) => s.to_string(),
            Token::Index(u) => u.to_string(),
        }
    }
}

impl TryFrom<&str> for Token {
    type Error = String;

    fn try_from(s: &str) -> Result<Token, Self::Error> {
        use std::str::FromStr;
        match usize::from_str(s) {
            Ok(u) => Ok(Token::Index(u)),
            Err(_) => Ok(Token::Key(s.to_string())),
        }
    }
}

pub trait Extractable {
    type Output: Sized;
    fn extract<'a>(&'a self, extractor: &[Token]) -> Option<&'a Self::Output>;
}

use tedge_lib::measurement::Measurement;
use tedge_lib::measurement::MeasurementValue;

impl Extractable for Measurement {
    type Output = MeasurementValue;

    fn extract<'a>(&'a self, extractor: &[Token]) -> Option<&'a Self::Output> {
        if let Some(next_token) = extractor.get(0) {
            match next_token {
                Token::Index(_) => None,
                Token::Key(key) => if key == self.name() {
                    self.value().extract(&extractor[1..])
                } else {
                    None
                }
            }
        } else {
            None
        }
    }
}

impl Extractable for tedge_lib::measurement::MeasurementValue {
    type Output = tedge_lib::measurement::MeasurementValue;

    fn extract<'a>(&'a self, extractor: &[Token]) -> Option<&'a Self::Output> {
        if let Some(next_token) = extractor.get(0) {
            match (next_token, self) {
                (Token::Index(idx), MeasurementValue::List(lst)) => {
                    trace!("Fetching '{}' from {:?}", idx, lst);
                    lst.get(*idx).and_then(|v| v.extract(&extractor[1..]))
                },
                (Token::Key(key), MeasurementValue::Map(map)) => {
                    trace!("Fetching '{}' from {:?}", key, map);
                    map.get(key).and_then(|v| v.extract(&extractor[1..]))
                },

                (_, MeasurementValue::Bool(_)) => None,
                (_, MeasurementValue::Float(_)) => None,
                (_, MeasurementValue::Text(_)) => None,
                (Token::Index(_), _) => None,
                (Token::Key(_), _) => None,
            }
        } else {
            trace!("Found value: {:?}", self);
            Some(self)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_extracting_empty_string() {
        assert!(Extractor::from_str("").is_err());
    }

    #[test]
    fn test_extracting_single_key() {
        let ext = Extractor::from_str("foo").unwrap().0;
        let exp = vec![Token::Key("foo".to_string())];
        assert_eq!(ext, exp);
    }

    #[test]
    fn test_extracting_key_index() {
        let ext = Extractor::from_str("foo.5").unwrap().0;
        let exp = vec![Token::Key("foo".to_string()), Token::Index(5)];
        assert_eq!(ext, exp);
    }

    #[test]
    fn test_extracting_key_index_key() {
        let ext = Extractor::from_str("foo.5.bar").unwrap().0;
        let exp = vec![Token::Key("foo".to_string()), Token::Index(5), Token::Key("bar".to_string())];
        assert_eq!(ext, exp);
    }
}
