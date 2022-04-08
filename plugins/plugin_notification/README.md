# plugin_notification

This plugin receives measurements that it forwards to another plugin.
Every time it receives a measurement, a third plugin is notified.


## Configuration

Example configuration of the plugin:

```toml
forward_to = "other_plugin_name"
notify = "to_notify_plugin_name"
raise = "info" # one of "info", "warning", "error"
raise_message = "Some freeform text"
```

