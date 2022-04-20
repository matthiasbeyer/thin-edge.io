use miette::Result;

#[derive(Debug, miette::Diagnostic, thiserror::Error)]
#[error("Error while shutting down MQTT Plugin")]
pub struct MqttShutdownError {
    #[related]
    errs: Vec<miette::Error>
}

impl MqttShutdownError {
    pub fn build_for(clientres: Result<()>, stopres: Result<()>) -> std::result::Result<(), Self> {
        let mut errs = Vec::with_capacity(2);
        if let Err(e) = clientres {
            errs.push(e);
        }

        if let Err(e) = stopres {
            errs.push(e);
        }

        if errs.is_empty() {
            Ok(())
        } else {
            Err(Self {
                errs
            })
        }
    }
}
