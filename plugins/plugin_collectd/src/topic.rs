use crate::error::Error;

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct CollectdTopic<'a> {
    pub(crate) metric_group_key: &'a str,
    pub(crate) metric_key: &'a str,
}

impl<'a> CollectdTopic<'a> {
    pub fn parse(s: &'a str) -> Result<Self, Error> {
        let mut iter = s.split('/');
        let _collectd_prefix = iter.next().ok_or_else(|| Error::InvalidCollectdTopicName(s.to_string()))?;
        let _hostname = iter.next().ok_or_else(|| Error::InvalidCollectdTopicName(s.to_string()))?;
        let metric_group_key = iter.next().ok_or_else(|| Error::InvalidCollectdTopicName(s.to_string()))?;
        let metric_key = iter.next().ok_or_else(|| Error::InvalidCollectdTopicName(s.to_string()))?;

        match iter.next() {
            None => Ok(CollectdTopic {
                metric_group_key,
                metric_key,
            }),
            Some(_) => Err(Error::InvalidCollectdTopicName(s.to_string())),
        }
    }
}

