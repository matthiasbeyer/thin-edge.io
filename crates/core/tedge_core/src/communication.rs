use std::any::TypeId;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

use tedge_api::address::MessageSender;
use tedge_api::error::DirectoryError;
use tedge_api::plugin::PluginDirectory as ApiPluginDirectory;
use tedge_api::Address;

use crate::errors::TedgeApplicationError;

pub struct CorePluginDirectory {
    plugins: HashMap<String, PluginInfo>,
    sender: MessageSender,
}

impl CorePluginDirectory {
    pub(crate) fn collect_from<I>(iter: I, sender: MessageSender) -> Result<Self, TedgeApplicationError>
        where I: std::iter::IntoIterator<Item = Result<(String, PluginInfo), TedgeApplicationError>>
    {
        Ok(CorePluginDirectory {
            plugins: iter.into_iter().collect::<Result<HashMap<_, _>, _>>()?,
            sender,
        })
    }

    pub(crate) fn get_mut<S: AsRef<str>>(&mut self, name: S) -> Option<&mut PluginInfo> {
        self.plugins.get_mut(name.as_ref())
    }

    fn get_address_for<MB: tedge_api::address::ReceiverBundle>(
        &self,
        name: &str,
    ) -> Result<Address<MB>, DirectoryError> {
        let types = MB::get_ids().into_iter().collect();

        let plug = self
            .plugins
            .get(name)
            .ok_or_else(|| DirectoryError::PluginNameNotFound(name.to_string()))?;

        if !plug.types.is_superset(&types) {
            let unsupported_types = types.difference(&plug.types).map(|tpl| tpl.0).collect();
            Err(DirectoryError::PluginDoesNotSupport(name.to_string(), unsupported_types))
        } else {
            Ok(Address::new(plug.sender.clone()))
        }
    }

    fn get_address_for_core(&self) -> Address<tedge_api::CoreMessages> {
        Address::new(self.sender.clone())
    }

    pub fn for_plugin_named(self: Arc<Self>, plugin_name: &str) -> PluginDirectory {
        PluginDirectory {
            core: self.clone(),
            plugin_name: plugin_name.to_string(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct PluginInfo {
    pub(crate) types: HashSet<(&'static str, TypeId)>,
    pub(crate) receiver: Option<tedge_api::address::MessageReceiver>,
    pub(crate) sender: tedge_api::address::MessageSender,
}

impl PluginInfo {
    pub(crate) fn new(types: HashSet<(&'static str, TypeId)>, channel_size: usize) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::channel(channel_size);
        Self {
            types,
            receiver: Some(receiver),
            sender,
        }
    }
}

impl Clone for PluginInfo {
    fn clone(&self) -> Self {
        PluginInfo {
            types: self.types.clone(),
            receiver: None,
            sender: self.sender.clone(),
        }
    }
}

pub struct PluginDirectory {
    core: Arc<CorePluginDirectory>,
    plugin_name: String,
}

impl ApiPluginDirectory for PluginDirectory {
    fn get_address_for<RB: tedge_api::address::ReceiverBundle>(&self, name: &str) -> Result<Address<RB>, DirectoryError> {
        self.core.get_address_for::<RB>(name)
    }

    fn get_address_for_core(&self) -> Address<tedge_api::CoreMessages> {
        self.core.get_address_for_core()
    }

    fn get_address_for_self<RB: tedge_api::address::ReceiverBundle>(&self) -> Result<Address<RB>, DirectoryError> {
        self.core.get_address_for::<RB>(&self.plugin_name)
    }
}
