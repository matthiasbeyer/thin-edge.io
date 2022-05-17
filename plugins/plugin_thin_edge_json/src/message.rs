use thin_edge_json::data::ThinEdgeJson;

#[derive(Debug)]
pub struct ThinEdgeJsonMessage(ThinEdgeJson);

impl From<ThinEdgeJson> for ThinEdgeJsonMessage {
    fn from(tejson: ThinEdgeJson) -> Self {
        Self(tejson)
    }
}

impl ThinEdgeJsonMessage {
    pub fn inner(&self) -> &ThinEdgeJson {
        &self.0
    }

    pub fn into_inner(self) -> ThinEdgeJson {
        self.0
    }
}

impl tedge_api::plugin::Message for ThinEdgeJsonMessage {}
