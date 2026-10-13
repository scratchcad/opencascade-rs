# scratchcad-occt-rs

Script the [OpenCASCADE](https://dev.opencascade.org) CAD kernel with [Rhai](https://rhai.rs).
Write a short script and get a real B-rep solid you can export to STEP, IGES, BREP or STL.

```rhai
// mounting_plate.rhai
let plate = box_centered(80, 50, 6).fillet(5, "|Z");

for x in [-1, 1] {
    for y in [-1, 1] {
        plate -= cylinder([x * 32, y * 17, -6], "Z", 2.5, 12);
    }
}

plate.chamfer(0.8, ">Z")
```

```console
$ occt-rhai mounting_plate.rhai -o plate.step
Shape(solid, 26 faces, 60 edges, 36 vertices)
BoundingBox(min: (-40, -25, -3), max: (40, 25, 3.000000000000002))
volume: 23297.832215
wrote plate.step
```

## Crates

| crate | what it is |
|-------|------------|
| [`scratchcad-occt-rhai`](https://github.com/scratchcad/scratchcad-occt-rs/tree/main/crates/scratchcad-occt-rhai) | The Rhai scripting API, a matching Rust API, and the `occt-rhai` command line tool. |
| [`scratchcad-occt-sys`](https://github.com/scratchcad/scratchcad-occt-rs/tree/main/crates/scratchcad-occt-sys) | Low-level [cxx](https://cxx.rs) bindings to OpenCASCADE 7.8 that turn C++ exceptions into Rust `Result`s. |

OpenCASCADE itself is built from source by the [`occt-sys`](https://crates.io/crates/occt-sys)
crate, or linked from a system installation.

## Usage

### Command line

```sh
cargo install scratchcad-occt-rhai
occt-rhai part.rhai -o part.step   # or .stp, .iges, .brep, .stl
```

The script's last expression is the shape to export. Run `occt-rhai --help` for the options.

### From Rust

```sh
cargo add scratchcad-occt-rhai
```

Run a script with `eval_shape`, or add the functions to your own Rhai `Engine` with `register`:

```rust
let part = scratchcad_occt_rhai::eval_shape(r#"box(40, 30, 5).fillet(2, "|Z")"#)?;
part.write_step("plate.step")?;
```

The same operations are available as a Rust API:

```rust
use scratchcad_occt_rhai::{DVec3, Shape};

let plate = Shape::make_box(40.0, 30.0, 5.0)?;
let hole = Shape::cylinder(4.0, 5.0)?.translate(DVec3::new(20.0, 15.0, 0.0))?;
let part = plate.fillet(2.0, plate.edges().select("|Z")?.shapes())?.cut(&hole)?;
```

### Learn more

- [Rhai API reference](https://github.com/scratchcad/scratchcad-occt-rs/blob/main/docs/rhai-api.md):
  every function, selectors and sketches.
- [Example scripts](https://github.com/scratchcad/scratchcad-occt-rs/tree/main/crates/scratchcad-occt-rhai/examples).

## Building

You need a Rust toolchain, CMake and a C++ compiler. By default OpenCASCADE is compiled from
source, which takes roughly 20 minutes the first time. To use a system installation of
OpenCASCADE 7.8 instead, build with `--no-default-features` (and set `DEP_OCCT_ROOT` to its install
prefix if CMake cannot find it).

Run the tests with `cargo test --workspace`.

## Credits

The `scratchcad-occt-sys` bindings started as a fork of `opencascade-sys` from
[bschwind/opencascade-rs](https://github.com/bschwind/opencascade-rs).

## License

LGPL-2.1, the same as OpenCASCADE.
