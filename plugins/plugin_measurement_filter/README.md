# plugin_measurement_filter

The "measurement_filter" plugin can be used to extract values from measurement
messages and apply a filter function on them. If the filter matches, the plugin
forwards the message to a certain plugin, if not it can optionally send the
message to an alternative plugin.


## Configuration

The plugin configuration is made out of four values:

* The target plugin to send messages to that are filtered
* An optional alternative plugin that messages get send to that are "filtered
  out"
* An extractor, that must be used to extract a value from a measurement message
  for filtering
* A filter predicate

An example would look like this:

```toml
target = "logger"
filtered_target = "some_other_plugin" # optional

# Extract the value from messages named "temperature" at field "fahrenheit"
extractor = "temperature.fahrenheit"

# Messages with fahrenheit > 70 are send to "target", all others to
# "filtered_target" or dropped
more_than = 70.0
```


## Available filter predicates

```toml
# Boolean
is = true

# Float
less_than = 10.0
more_than = 10.0

# String
contains = "foo"
excludes = "foo"
```

One of them must be used, using multiple is not supported.

If a filter does not match the expected type (e.g. the value to filter is a
boolean, but you try to filter with `more_than = 10.0`) the filter
implementation will return false.

