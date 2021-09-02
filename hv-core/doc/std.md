Lua API documentation.

Most crates in the Heavy family come with Lua APIs; `hv-core` is no exception. It comes bundled with
several convenient pieces of functionality as well as Lua bindings for its most important core Rust
types.

## std.agent

A simple class for message-passing based state machines.

## std.binser

Calvin Rose's `binser` Lua library for serializing Lua objects to byte strings. `binser` is not just
included for doing any sort of serialization you might need; it's also used when serializing spaces
through the functions included in the [`spaces::serialize`](crate::spaces::serialize) module. It's
how we serialize Lua objects which are included in a space. In addition, `binser` has a great
compatibility feature with the 30log library (`std.class`) where you can register classes and avoid
having to deal with serializing their metatables that way. If you have a Lua table which is an
object table in a `Space` which is going to be serialized, and it's an instance of a class, you
almost certainly want that class to be registered with `binser.registerClass`.

## std.class

Roland Yonaba's 30log object orientation framework for Lua. It's useful for doing
things like creating "classes" to wrap your objects in, and crates like `hv-friends` even include
"mixins" compatible with 30log for doing things like adding standard functions for accessing
`Position` and `Velocity` components. 30log is simple and flexible, and while object oriented
programming doesn't solve all problems, it can add structure to working with Lua.

## std.space

Contains two useful classes to use as base classes for holding your [`Object`]s and [`Space`]s in
your Lua code, with the intent that you can extend them and then add mixins from libraries such as
`hv-friends` to give an object table functionality related to the object it represents.
