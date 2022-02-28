# plugin_avg

This showcases a rather simplistic "avg" plugin, that collects incoming
measuremens (messages of kind `Measurement`) and periodically sends out an
average of the collected values of the last timeframe.

## Note

Currently, only integer measurements are supported.


## Configuration

The configuration of the plugin can have the following fields

* `timeframe`: How long to collect messages before sending them out.
  E.G.: "1min"
* `target`: Whom to send the average to
* `report_on_zero_elements`: If there have not been any incoming measurements in
  the `timeframe`, whether to send out a zero, or not send anything
* `int_to_float_avg`: whether to send out float values, even though the recorded
  values are all integer.
  With this setting to `true`, `[1, 2]` will be send out as `1.5` rather than
  performing integer operations resulting in possibly inaccurate values


Example configuration:

```toml
# For a reference what format is supported here, please see
# https://docs.rs/humantime/latest/humantime/
timeframe = "1min"

target = "my_other_plugin"

report_on_zero_elements = false
int_to_float_avg = false
```

