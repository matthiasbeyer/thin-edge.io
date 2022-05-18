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

    impl Install {
        pub fn new(package_name: String) -> Self {
            Self {
                package_name
            }
        }
    }

    impl tedge_api::Message for Install {}

    /// Update a software by name
    #[derive(Debug, getset::Getters)]
    pub struct Update {
        /// The name of the package in this operation
        #[getset(get = "pub")]
        package_name: String,
    }

    impl Update {
        pub fn new(package_name: String) -> Self {
            Self {
                package_name
            }
        }
    }

    impl tedge_api::Message for Update {}

    /// Uninstall a software by name
    #[derive(Debug, getset::Getters)]
    pub struct Uninstall {
        /// The name of the package in this operation
        #[getset(get = "pub")]
        package_name: String,
    }

    impl Uninstall {
        pub fn new(package_name: String) -> Self {
            Self {
                package_name
            }
        }
    }

    impl tedge_api::Message for Uninstall {}
}

/// Types for representing a "response" that was yielded by an operation for a "request"
pub mod response {
    /// A list of installed things
    #[derive(Debug)]
    pub enum ListResponse {
        List {
            list: Vec<String>,
        },

        ListFailed {
            message: String
        }
    }

    impl tedge_api::Message for ListResponse {}

    #[derive(Debug)]
    pub enum InstallResponse {
        InstallProgress {
            package_name: String,
            progress: usize,
        },

        InstallLogLine {
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
    }

    impl tedge_api::Message for InstallResponse {}

    #[derive(Debug)]
    pub enum UpdateResponse {
        UpdateProgress {
            package_name: String,
            progress: usize,
        },

        UpdateLogLine {
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
    }

    impl tedge_api::Message for UpdateResponse {}


    #[derive(Debug)]
    pub enum UninstallResponse {
        UninstallProgress {
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

    impl tedge_api::Message for UninstallResponse {}
}
