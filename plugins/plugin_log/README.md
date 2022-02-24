# plugin_log

This showcases a _very_ simplistic "log" plugin, that does nothing more than
logging incoming messages.


## Configuration

The configuration of the plugin is as simple as possible:

* Which "level" should be used for logging (trace, debug, info, warn, error)
* Whether to acknowledge incoming messages, that means send back a reply
* Whether to instantiate the process-wide logger (this is because inside a rust
  process, there can only be one logger instance). This should normally be kept
  off

Example configuration:

```toml
level = "debug"
acknowledge = true
setup_logger = false
```

