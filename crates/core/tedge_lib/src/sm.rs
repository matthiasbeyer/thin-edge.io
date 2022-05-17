/// Types for requesting software-management operations
pub mod request {
    /// List installed software
    #[derive(Debug)]
    pub struct List;

    impl tedge_api::Message for List {}

    /// Install a software by name
    #[derive(Debug, getset::Getters)]
    pub struct Install {
        /// The name of the package in this operation
        #[getset(get = "pub")]
        package_name: String,
    }

    impl tedge_api::Message for Install {}

    /// Update a software by name
    #[derive(Debug, getset::Getters)]
    pub struct Update {
        /// The name of the package in this operation
        #[getset(get = "pub")]
        package_name: String,
    }

    impl tedge_api::Message for Update {}

    /// Uninstall a software by name
    #[derive(Debug, getset::Getters)]
    pub struct Uninstall {
        /// The name of the package in this operation
        #[getset(get = "pub")]
        package_name: String,
    }

    impl tedge_api::Message for Uninstall {}
}

/// Types for representing a "response" that was yielded by an operation for a "request"
pub mod response {
    /// A list of installed things
    #[derive(Debug, getset::Getters)]
    pub struct List {
        #[getset(get = "pub")]
        list: Vec<String>,
    }

    impl tedge_api::Message for List {}

    /// Listing installed things failed
    #[derive(Debug)]
    pub struct ListFailed;

    impl tedge_api::Message for ListFailed {}

    /// A state representing an ongoing install process
    #[derive(Debug, getset::Getters, getset::CopyGetters)]
    pub struct InstallingState {
        /// The name of the package in this operation
        #[getset(get = "pub")]
        package_name: String,

        ///
        /// A number between 0 and 100 describing the progress of the operation
        #[getset(get_copy = "pub")]
        progress: usize,
    }

    impl tedge_api::Message for InstallingState {}

    /// A log line from an install process
    #[derive(Debug, getset::Getters)]
    pub struct InstallingLogLine {
        /// The name of the package in this operation
        #[getset(get = "pub")]
        package_name: String,

        /// A single line of output from the procedure
        #[getset(get = "pub")]
        log_line: String,
    }

    impl tedge_api::Message for InstallingLogLine {}

    /// Installing a package succeeded
    #[derive(Debug, getset::Getters)]
    pub struct InstallSucceeded {
        /// The name of the package in this operation
        #[getset(get = "pub")]
        package_name: String,
    }

    impl tedge_api::Message for InstallSucceeded {}

    /// Installing a package failed
    #[derive(Debug, getset::Getters)]
    pub struct InstallFailed {
        /// The name of the package in this operation
        #[getset(get = "pub")]
        package_name: String,

        /// A human-readable message describing the failure
        #[getset(get = "pub")]
        failure_message: String,
    }

    impl tedge_api::Message for InstallFailed {}

    ///
    /// Progress report from an ongoing update process
    #[derive(Debug, getset::Getters, getset::CopyGetters)]
    pub struct UpdatingState {
        /// The name of the package in this operation
        #[getset(get = "pub")]
        package_name: String,

        /// A number between 0 and 100 describing the progress of the operation
        #[getset(get_copy = "pub")]
        progress: usize,
    }

    impl tedge_api::Message for UpdatingState {}

    /// A log line from an ongoing update process
    #[derive(Debug, getset::Getters)]
    pub struct UpdatingLogLine {
        /// The name of the package in this operation
        #[getset(get = "pub")]
        package_name: String,

        /// A single line of output from the procedure
        #[getset(get = "pub")]
        log_line: String,
    }

    impl tedge_api::Message for UpdatingLogLine {}

    /// A update process succeeded
    #[derive(Debug, getset::Getters)]
    pub struct UpdateSucceeded {
        /// The name of the package in this operation
        #[getset(get = "pub")]
        package_name: String,
    }

    impl tedge_api::Message for UpdateSucceeded {}

    /// A update process failed
    #[derive(Debug, getset::Getters)]
    pub struct UpdateFailed {
        /// The name of the package in this operation
        #[getset(get = "pub")]
        package_name: String,

        /// A human-readable message describing the failure
        #[getset(get = "pub")]
        failure_message: String,
    }

    impl tedge_api::Message for UpdateFailed {}

    ///
    /// Progress report from an ongoing uninstall process
    #[derive(Debug, getset::Getters, getset::CopyGetters)]
    pub struct UninstallState {
        /// The name of the package in this operation
        #[getset(get = "pub")]
        package_name: String,

        /// A number between 0 and 100 describing the progress of the operation
        #[getset(get_copy = "pub")]
        progress: usize,
    }

    impl tedge_api::Message for UninstallState {}

    /// A log line from an ongoing uninstall process
    #[derive(Debug, getset::Getters)]
    pub struct UninstallLogLine {
        /// The name of the package in this operation
        #[getset(get = "pub")]
        package_name: String,

        /// A single line of output from the procedure
        #[getset(get = "pub")]
        log_line: String,
    }

    impl tedge_api::Message for UninstallLogLine {}

    /// Uninstall process succeeded
    #[derive(Debug, getset::Getters)]
    pub struct UninstallSucceeded {
        /// The name of the package in this operation
        #[getset(get = "pub")]
        package_name: String,
    }

    impl tedge_api::Message for UninstallSucceeded {}

    /// Uninstall process failed
    #[derive(Debug, getset::Getters)]
    pub struct UninstallFailed {
        /// The name of the package in this operation
        #[getset(get = "pub")]
        package_name: String,

        /// A human-readable message describing the failure
        #[getset(get = "pub")]
        failure_message: String,
    }

    impl tedge_api::Message for UninstallFailed {}
}
