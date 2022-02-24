# plugin_sysstat

This showcases a rather _complex_ "sysstat" plugin, that collects system
statistics data and sends it out in configured intervals.


## Configuration

**Note**: The example configuration of this module is quite long, because it
covers all settings the module can have.

**Subsections for the individual backends are optional**, so if you only want
the `cpu` backend to be active, you *don't* have to specify all other backends and
`enable = false` them.

The configuration of the plugin can have the following fields

```toml
# section for the memory-statistics backend
[memory]

# (bool) whether to enable collecting these statistics
enable = true

# (nonzero integer) interval in which to report these statistics
interval_ms = 100

# (bool) whether to report the total memory
total_memory = true

# (String) The name under which the measurement is reported
# if not specified: `total_memory`
total_memory_name = "total_memory"

# (bool) Whether to report free_memory measurements
free_memory = true

# (String) The name under which the measurement is reported
# if not specified: `free_memory`
free_memory_name = "total_memory"

# (bool) Whether to report available_memory measurements
available_memory = true

# (String) The name under which the measurement is reported
# if not specified: `available_memory`
available_memory_name = "total_memory"

# (bool) Whether to report used_memory measurements
used_memory = true

# (String) The name under which the measurement is reported
# if not specified: `used_memory`
used_memory_name = "total_memory"

# (bool) Whether to report free_swap measurements
free_swap = true

# (String) The name under which the measurement is reported
# if not specified: `free_swap`
free_swap_name = "total_memory"

# (bool) Whether to report used_swap measurements
used_swap = true

# (String) The name under which the measurement is reported
# if not specified: `used_swap`
used_swap_name = "total_memory"


# section for the network-statistics backend
[network]
# (bool) whether to enable collecting these statistics
enable = true

# (nonzero integer) interval in which to report these statistics
interval_ms = 100

# Subsection for configuration that applies to all network statistics
[network.all_networks]
# (bool) whether to enable collecting these statistics
enable = true

# The other supported keys in this section are the very same as in the
# `network.by_name` section!

# Subsection for configuring networks by name.
# An example for "<name>" here would be "eth0"
[network.by_name.<name>]
# Enable to report the "received" measurement
received = true

# Name to use to report the "received" measurement
# if not specified: "received"
received_name = "received"

# Enable to report the "total_received" measurement
total_received = true

# Name to use to report the "total_received" measurement
# if not specified: "total_received"
total_received_name = "total_received"

# Enable to report the "transmitted" measurement
transmitted = true

# Name to use to report the "transmitted" measurement
# if not specified: "transmitted"
transmitted_name = "transmitted"

# Enable to report the "total_transmitted" measurement
total_transmitted = true

# Name to use to report the "total_transmitted" measurement
# if not specified: "total_transmitted"
total_transmitted_name = "total_transmitted"

# Enable to report the "packets_received" measurement
packets_received = true

# Name to use to report the "packets_received" measurement
# if not specified: "packets_received"
packets_received_name = "packets_received"

# Enable to report the "total_packets_received" measurement
total_packets_received = true

# Name to use to report the "total_packets_received" measurement
# if not specified: "total_packets_received"
total_packets_received_name = "total_packets_received"

# Enable to report the "packets_transmitted" measurement
packets_transmitted = true

# Name to use to report the "packets_transmitted" measurement
# if not specified: "packets_transmitted"
packets_transmitted_name = "packets_transmitted"

# Enable to report the "total_packets_transmitted" measurement
total_packets_transmitted = true

# Name to use to report the "total_packets_transmitted" measurement
# if not specified: "total_packets_transmitted"
total_packets_transmitted_name = "total_packets_transmitted"

# Enable to report the "errors_on_received" measurement
errors_on_received = true

# Name to use to report the "errors_on_received" measurement
# if not specified: "errors_on_received"
errors_on_received_name = "errors_on_received"

# Enable to report the "total_errors_on_received" measurement
total_errors_on_received = true

# Name to use to report the "total_errors_on_received" measurement
# if not specified: "total_errors_on_received"
total_errors_on_received_name = "total_errors_on_received"


# section for the cpu-statistics backend
[cpu]
# (bool) whether to enable collecting these statistics
enable = true

# (nonzero integer) interval in which to report these statistics
interval_ms = 100

# Name to use to report the "global_processor_info" measurement
# if not specified: "global_processor_info"
global_processor_info_name = "global_processor_info"

# Name to use to report the "processor_info" measurement
# if not specified: "processor_info"
processor_info_name = "processor_info"

# Name to use to report the "physical_core_count" measurement
# if not specified: "physical_core_count_name"
physical_core_count_name = "physical_core_count"

#[serde(default = "physical_core_count_name_default")]
pub(crate) physical_core_count_name: String,

[cpu.report_global_processor_info]
# same as [cpu.report_processor_info], but reports global processor statistics

[cpu.report_processor_info]
# Enable reporting this stat
enable = true

# Report the processor frequency
frequency = true

# Report the processor frequency measurement with this name
frequency_name = "frequency"

# Report the processor cpu usage
cpu_usage = true

# Report the processor cpu usage measurement with this name
cpu_usage_name = "cpu_usage"

# Report the processor name
name = true

# Report the processor name measurement with this name
name_name = "name"

# Report the processor vendor id
vendor_id = true

# Report the processor vendor id measurement with this name
vendor_id_name = "vendor_id"

# Report the processor brand
brand = true

# Report the processor brand measurement with this name
brand_name = "brand"


[cpu.report_physical_core_count]
# (bool) whether to enable collecting these statistics
enable = true


# section for the "disk usage"-statistics backend
[disk_usage]
# (bool) whether to enable collecting these statistics
enable = true

# (nonzero integer) interval in which to report these statistics
interval_ms = 100


# section for the load-statistics backend
[load]
# (bool) whether to enable collecting these statistics
enable = true

# (nonzero integer) interval in which to report these statistics
interval_ms = 100


# section for the process-statistics backend
[process]
# (bool) whether to enable collecting these statistics
enable = true

# (nonzero integer) interval in which to report these statistics
interval_ms = 100

[process.all_processes]
# (bool) whether to enable collecting these statistics
enable = true

# The other supported keys in this section are the very same as in the
# `process.by_name` section!

# Subsection for configuring process-statistics gathering by name.
# An example for "<name>" here would be "bash"
[process.by_name.<name>]
# Whether to collect the command itself
cmd = true

# Whether to collect the current working directory
cwd = true

# Whether to collect the cpu usage
cpu_usage = true

# Whether to collect the disk usage
disk_usage = true

# Whether to collect the executable path
exe = true

# Whether to collect the memory usage
memory = true

# Whether to collect the name of the process
name = true

# Whether to collect the parent PID
parent = true

# Whether to collect the PID
pid = true

# Whether to collect the root of the process
root = true

# Whether to collect the runtime
run_time = true

# Whether to collect the start-time
start_time = true

# Whether to collect the virtual memory usage
vmemory = true
```

