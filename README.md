# Heavy: a lightweight but absolutely crushing game development framework in Rust and Lua

Heavy is a lightweight cross-platform game development framework written in Rust with a focus on
efficient and easy Lua integration. It is currently in heavy development (heh) and is not considered
stable in any way shape or form. Heavy currently takes the form of a family of crates:

- Heavy core (`hv-core`), the core context acquisition/event handling/Lua integration/ECS crate
- Heavy Friends (`hv-friends`), a Love2D-like interface built on Heavy core
- MyMachine (`hv-mymachine`), a small "console" library that allows interfacing w/ a running game's
  Lua state from stdin
- Heavy Rain (`hv-rain`), a library for spawning and controlling 2D danmaku patterns
- Talisman (`hv-talisman`), a highly work-in-progress and experimental in-engine preview/level
  editing tool which keeps getting its entire source deleted as we figure out how to do it right
- `hv-fmod`, a library providing FMOD bindings for Heavy both to Lua and Rust
- `hv-egui`, a library providing a thin wrapper over the `egui` crate for making simple GUIs for
  development purposes

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.