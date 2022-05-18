//! Types how the SM request and response should look like in JSON

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type")]
pub enum SmRequest {
    List,

    Install {
        package_name: String,
    },

    Update {
        package_name: String,
    },

    Uninstall {
        package_name: String,
    },
}

#[derive(Debug, serde::Serialize)]
pub enum SmResponse {
    List {
        list: Vec<String>,
    },

    ListFailed {
        message: String
    },

    InstallingState {
        package_name: String,
        progress: usize,
    },

    InstallingLogLine {
        package_name: String,
        log_line: String,
    },

    InstallSucceeded {
        package_name: String,
    },

    InstallFailed {
        package_name: String,
        failure_message: String,
    },

    UpdatingState {
        package_name: String,
        progress: usize,
    },

    UpdatingLogLine {
        package_name: String,
        log_line: String,
    },

    UpdateSucceeded {
        package_name: String,
    },

    UpdateFailed {
        package_name: String,
        failure_message: String,
    },

    UninstallState {
        package_name: String,
        progress: usize,
    },

    UninstallLogLine {
        package_name: String,
        log_line: String,
    },

    UninstallSucceeded {
        package_name: String,
    },

    UninstallFailed {
        package_name: String,
        failure_message: String,
    },
}
