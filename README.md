# Zooid Lake

## Building

See Bevy's dependencies at [Bevy Quick Start](https://bevy.org/learn/quick-start/getting-started/setup/#installing-os-dependencies), some of which are required to build this crate.

The [Wild](https://github.com/wild-linker/wild) linker is used by default on Linux. This is done simply to improve link times, and you can use your toolchain's default linker by commenting out lines 4 and 5 in [.cargo/config.toml](.cargo/config.toml).

### Cargo Features

* `wayland` - Enable Linux target support with Wayland.
* `x11` - Enable Linux target support with X11.
* `webgl2` - Enable WASM target support with WebGL2.
* `webgpu` - Enable WASM target support with WebGPU.
* `debug` - Enable extended validation, detailed logging in standard IO instead of a file, and a console window on Windows. WASM builds never log to a file regardless of this feature.
