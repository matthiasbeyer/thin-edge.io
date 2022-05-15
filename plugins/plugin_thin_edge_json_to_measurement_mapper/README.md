# plugin_thin_edge_json_to_measurement_mapper

This plugin implements a mapper from the the "thin-edge-json" format to
`Measurement`s.

This plugin parses `ThinEdgeJson` messages it receives to `Measurement`
objects, which it then sends out to another plugin able to receive this kind of
objects.

## TODO

If the `ThinEdgeJson` object contains multiple measurements, these are currently
send out as individual `Measurement` objects.

This has to be cleaned up, depending on whether this is feasible.

## Configuration

The only configuration this plugin needs is the name of the plugin to send the
`ThinEdgeJson` objects to:

```toml
target = "my_other_plugin"
```


