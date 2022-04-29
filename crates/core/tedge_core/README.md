# tedge_core

This is the "core" implementation of tedge.

This means that this crate provides a runtime implementation that is used to
orchestrate the running of the individual components of thin-edge.io, called
"Plugins".

This crate can then be used in a very minimal CLI implementation to start up a
process and run all the domain specific parts of thin-edge.io.

