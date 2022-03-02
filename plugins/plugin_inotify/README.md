# plugin_inotify

This plugin can be used to watch files with inotify and send events to other
plugins.


## Configuration

The configuration for this plugin supports the following settings:

```toml
# The Plugin to send the events to
target = "my_other_plugin"

# Whether to error in the plugin, when inotify returns an error
# If the plugin itself errors, these will always be returned
# Defaults to true
fail_on_err = false

# The following section contains a map from Path -> List of watch modes
# For a list of supported watch modes, see below
[pathes]
"/etc/passwd" = ["ACCESS", "CLOSE_WRITE"]
```

## Message format

The plugin reports all events as Measurements of type String.
The following strings are reported:

* `"ACCESS"`
* `"ATTRIB"`
* `"CLOSE_WRITE"`
* `"CLOSE_NOWRITE"`
* `"CREATE"`
* `"DELETE"`
* `"DELETE_SELF"`
* `"MODIFY"`
* `"MOVE_SELF"`
* `"MOVED_FROM"`
* `"MOVED_TO"`
* `"OPEN"`
* `"IGNORED"`
* `"ISDIR"`
* `"Q_OVERFLOW"`
* `"UNMOUNT"`

Also, `"unknown"` is reported if the event cannot be identified.


## Supported watch modes

### ACCESS

File was accessed
When watching a directory, this event is only triggered for objects inside the directory,
not the directory itself.


### ATTRIB

Metadata (permissions, timestamps, …) changed
When watching a directory, this event can be triggered for the directory itself,
as well as objects inside the directory.


### CLOSE_WRITE

File opened for writing was closed
When watching a directory, this event is only triggered for objects inside the
directory, not the directory itself.


### CLOSE_NOWRITE

File or directory not opened for writing was closed
When watching a directory, this event can be triggered for the directory itself,
as well as objects inside the directory.


### CREATE

File/directory created in watched directory
When watching a directory, this event is only triggered for objects inside the
directory, not the directory itself.


### DELETE

File/directory deleted from watched directory
When watching a directory, this event is only triggered for objects inside the
directory, not the directory itself.


### DELETE_SELF

Watched file/directory was deleted


### MODIFY

File was modified
When watching a directory, this event is only triggered for objects inside the
directory, not the directory itself.


### MOVE_SELF

Watched file/directory was moved


### MOVED_FROM

File was renamed/moved; watched directory contained old name
When watching a directory, this event is only triggered for objects inside the
directory, not the directory itself.


### MOVED_TO

File was renamed/moved; watched directory contains new name
When watching a directory, this event is only triggered for objects inside the
directory, not the directory itself.


### OPEN

File or directory was opened
When watching a directory, this event can be triggered for the directory itself,
as well as objects inside the directory.


### ALL_EVENTS

Watch for all events
This constant is simply a convenient combination of the following other
constants:

* ACCESS
* ATTRIB
* CLOSE_WRITE
* CLOSE_NOWRITE
* CREATE
* DELETE
* DELETE_SELF
* MODIFY
* MOVE_SELF
* MOVED_FROM
* MOVED_TO
* OPEN


### MOVE

Watch for all move events
This constant is simply a convenient combination of the following other
constants:

* MOVED_FROM
* MOVED_TO


### CLOSE

Watch for all close events
This constant is simply a convenient combination of the following other
constants:

* CLOSE_WRITE
* CLOSE_NOWRITE


### DONT_FOLLOW

Don’t dereference the path if it is a symbolic link


### EXCL_UNLINK

Filter events for directory entries that have been unlinked


### MASK_ADD

If a watch for the inode exists, amend it instead of replacing it


### ONESHOT

Only receive one event, then remove the watch


### ONLYDIR

Only watch path, if it is a directory


