# plugin_thin_edge_json

This plugin implements the "thin-edge-json" format.

This plugin parses bytes it received from the mqtt plugin to
`ThinEdgeJson` objects and sends them to another plugin receiving this kind of
objects. It does _not_ implement MQTT itself.

## Configuration

The only configuration this plugin needs is the name of the plugin to send the
`ThinEdgeJson` objects to:

```toml
target = "my_other_plugin"
```

