use std::collections::HashMap;
use std::path::PathBuf;

#[derive(serde::Deserialize, Debug)]
pub struct InotifyConfig {
    /// Target to send notifications to
    pub(crate) target: String,

    /// Whether to error in the plugin, when inotify returns an error
    ///
    /// If the plugin itself errors, these will always be returned
    /// Defaults to true
    #[serde(default = "fail_on_err_default")]
    pub(crate) fail_on_err: bool,

    /// Pathes to watch
    pub(crate) pathes: HashMap<PathBuf, Vec<Watchmode>>,
}

fn fail_on_err_default() -> bool {
    true
}

#[derive(serde::Deserialize, Copy, Clone, Debug)]
#[allow(non_camel_case_types)]
pub enum Watchmode {
    /// File was accessed
    /// When watching a directory, this event is only triggered for objects inside the directory,
    /// not the directory itself.
    ACCESS,

    // Metadata (permissions, timestamps, …) changed
    //
    // When watching a directory, this event can be triggered for the directory itself, as well as objects inside the directory.
    ATTRIB,

    // File opened for writing was closed
    //
    // When watching a directory, this event is only triggered for objects inside the directory, not the directory itself.
    CLOSE_WRITE,

    // File or directory not opened for writing was closed
    //
    // When watching a directory, this event can be triggered for the directory itself, as well as objects inside the directory.
    CLOSE_NOWRITE,

    // File/directory created in watched directory
    //
    // When watching a directory, this event is only triggered for objects inside the directory, not the directory itself.
    CREATE,

    // File/directory deleted from watched directory
    //
    // When watching a directory, this event is only triggered for objects inside the directory, not the directory itself.
    DELETE,

    // Watched file/directory was deleted
    DELETE_SELF,

    // File was modified
    //
    // When watching a directory, this event is only triggered for objects inside the directory, not the directory itself.
    MODIFY,

    // Watched file/directory was moved
    MOVE_SELF,

    // File was renamed/moved; watched directory contained old name
    //
    // When watching a directory, this event is only triggered for objects inside the directory, not the directory itself.
    MOVED_FROM,

    // File was renamed/moved; watched directory contains new name
    //
    // When watching a directory, this event is only triggered for objects inside the directory, not the directory itself.
    MOVED_TO,

    // File or directory was opened
    //
    // When watching a directory, this event can be triggered for the directory itself, as well as objects inside the directory.
    OPEN,

    // Watch for all events
    //
    // This constant is simply a convenient combination of the following other constants:
    //
    //     ACCESS
    //     ATTRIB
    //     CLOSE_WRITE
    //     CLOSE_NOWRITE
    //     CREATE
    //     DELETE
    //     DELETE_SELF
    //     MODIFY
    //     MOVE_SELF
    //     MOVED_FROM
    //     MOVED_TO
    //     OPEN
    ALL_EVENTS,

    // Watch for all move events
    //
    // This constant is simply a convenient combination of the following other constants:
    //
    //     MOVED_FROM
    //     MOVED_TO
    MOVE,

    // Watch for all close events
    //
    // This constant is simply a convenient combination of the following other constants:
    //
    //     CLOSE_WRITE
    //     CLOSE_NOWRITE
    CLOSE,

    // Don’t dereference the path if it is a symbolic link
    DONT_FOLLOW,

    // Filter events for directory entries that have been unlinked
    EXCL_UNLINK,

    // If a watch for the inode exists, amend it instead of replacing it
    MASK_ADD,

    // Only receive one event, then remove the watch
    ONESHOT,

    // Only watch path, if it is a directory
    ONLYDIR,
}

impl From<Watchmode> for inotify::WatchMask {
    fn from(wm: Watchmode) -> Self {
        match wm {
            Watchmode::ACCESS => inotify::WatchMask::ACCESS,
            Watchmode::ATTRIB => inotify::WatchMask::ATTRIB,
            Watchmode::CLOSE_WRITE => inotify::WatchMask::CLOSE_WRITE,
            Watchmode::CLOSE_NOWRITE => inotify::WatchMask::CLOSE_NOWRITE,
            Watchmode::CREATE => inotify::WatchMask::CREATE,
            Watchmode::DELETE => inotify::WatchMask::DELETE,
            Watchmode::DELETE_SELF => inotify::WatchMask::DELETE_SELF,
            Watchmode::MODIFY => inotify::WatchMask::MODIFY,
            Watchmode::MOVE_SELF => inotify::WatchMask::MOVE_SELF,
            Watchmode::MOVED_FROM => inotify::WatchMask::MOVED_FROM,
            Watchmode::MOVED_TO => inotify::WatchMask::MOVED_TO,
            Watchmode::OPEN => inotify::WatchMask::OPEN,
            Watchmode::ALL_EVENTS => inotify::WatchMask::ALL_EVENTS,
            Watchmode::MOVE => inotify::WatchMask::MOVE,
            Watchmode::CLOSE => inotify::WatchMask::CLOSE,
            Watchmode::DONT_FOLLOW => inotify::WatchMask::DONT_FOLLOW,
            Watchmode::EXCL_UNLINK => inotify::WatchMask::EXCL_UNLINK,
            Watchmode::MASK_ADD => inotify::WatchMask::MASK_ADD,
            Watchmode::ONESHOT => inotify::WatchMask::ONESHOT,
            Watchmode::ONLYDIR => inotify::WatchMask::ONLYDIR,
        }
    }
}
