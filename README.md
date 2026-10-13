# opencascade-rhai

Script the [OpenCASCADE](https://dev.opencascade.org) CAD kernel with [Rhai](https://rhai.rs).

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
$ cargo run --release --bin occt-rhai -- mounting_plate.rhai -o plate.step
Shape(solid, 26 faces, 60 edges, 36 vertices)
BoundingBox(min: (-40, -25, -3), max: (40, 25, 3.000000000000002))
volume: 23297.832215
wrote plate.step
```

## Workspace

| crate | what it is |
|-------|------------|
| [`occt-sys`](https://crates.io/crates/occt-sys) | Used unchanged from crates.io (`=7.8.1`): builds OpenCASCADE 7.8.1 from source as static libraries. |
| [`crates/opencascade-sys`](crates/opencascade-sys) | A fork of `opencascade-sys` (cxx bindings) that adds C++ exception handling and the bindings the Rhai layer needs. |
| [`crates/opencascade-rhai`](crates/opencascade-rhai) | The high-level crate, designed around Rhai: cheap-to-clone handles, `Result`-returning methods, chainable calls and selectors. Includes the `occt-rhai` command line tool. |

### Exception handling in `opencascade-sys`

OpenCASCADE reports errors by throwing subclasses of `Standard_Failure`, which do not derive from
`std::exception`. cxx only catches `std::exception`, so in the upstream bindings any OCCT error
escapes through the FFI boundary and aborts the process.

The fork installs a custom `rust::behavior::trycatch` (in
[`include/bindings_common.hxx`](crates/opencascade-sys/include/bindings_common.hxx)) that catches
`Standard_Failure`, `std::exception` and anything else, and every binding that can throw returns
`Result<T, cxx::Exception>`. The error message is `"<OCCT exception type>: <message>"`, for example
`Standard_ConstructionError: gp_Dir() - input vector has zero norm`.

One subtlety: OCCT builds its release libraries with `-DNo_Exception`, which compiles out the
`*_Raise_if` checks *inside the libraries* (checks in inline headers still fire, since they compile
into the bindings). Where that would turn an error into a null handle or a crash — reading the
value of a failed `GC_MakeArcOfCircle`, making a segment between coincident points, reading a line
as a circle, projecting without a solution — the C++ helpers check explicitly and throw. Helpers
that dereference OCCT handles throw `Standard_NullObject` instead of dereferencing null.

## The Rhai API

`opencascade_rhai::engine()` returns a Rhai `Engine` with everything registered;
`opencascade_rhai::register(&mut engine)` adds it to an engine of your own. `eval_shape(script)`
runs a script and converts its result to a `Shape`.

### Conventions

* **Handles.** `Shape`, `Selection`, `Sketch` and `Plane` are immutable values that are cheap to
  clone. Every operation returns a new value, so calls chain and variables never change under you.
* **One shape type.** A `Shape` can be a vertex, edge, wire, face, shell, solid or compound;
  operations that need a particular kind check at runtime. Algorithms that return a compound
  holding a single shape return that shape directly.
* **Errors.** Every fallible function raises a runtime error that scripts can catch:
  `try { ... } catch (err) { print(err) }`.
* **Numbers** may be integers or floats. **Angles** are in degrees.
* **Vectors** are `vec3(x, y, z)` values or arrays (`[x, y, z]`, or `[x, y]` with `z = 0`).
  **Directions** may also be axis names: `"X"`, `"-Z"`.
* **Planes** may be `Plane` values or names (`"XY"`, `"YZ"`, `"ZX"`, `"XZ"`, `"YX"`, `"ZY"`).
* **Picking faces or edges.** Arguments like `fillet`'s edges accept a selector string, a
  `Selection`, a single `Shape` or an array of shapes.

### Constructors

| function | result |
|----------|--------|
| `box(dx, dy, dz)`, `box_centered(dx, dy, dz)`, `box_from_corners(a, b)`, `cube(s)` | solid box |
| `cylinder(r, h)`, `cylinder(base, axis, r, h)` | solid cylinder |
| `sphere(r)`, `sphere(center, r)`, `cone(r1, r2, h)`, `torus(R, r)` | solids |
| `vertex(p)`, `line(a, b)`, `arc(start, mid, end)` | vertex / edges |
| `circle(r)`, `circle(plane, r)`, `circle(center, normal, r)` | closed wire |
| `rect(w, h)`, `rect(plane, w, h)` | closed wire, centered |
| `polygon(points)`, `polygon(plane, points_2d)`, `polyline(points)` | closed / open wire |
| `spline(points)`, `spline(points, closed)`, `bezier(points)` | edges |
| `wire(edges)`, `face(wire)`, `face(outer, holes)`, `compound(shapes)` | assemblies |
| `fuse_all(shapes)`, `loft(sections)`, `loft(sections, solid)` | solids |
| `sketch()`, `sketch(plane)` | a `Sketch` (see below) |
| `vec3(x, y, z)`, `plane(name)`, `plane(origin, normal)`, `plane(origin, normal, x_dir)` | values |
| `load(path)`, `import_step(path)`, `import_iges(path)`, `import_brep(path)` | read a file |

### Shape methods

| category | methods |
|----------|---------|
| booleans | `fuse`/`union`/`+`, `cut`/`-`, `common`/`intersect`/`&`, `section` |
| transforms | `translate(v)`, `translate(x, y, z)`, `rotate(axis, deg)`, `rotate(origin, axis, deg)`, `rotate_x/y/z(deg)`, `scale(f)`, `scale(center, f)`, `mirror(plane_or_normal)`, `mirror(origin, normal)` |
| features | `extrude(distance)` (along the face normal), `extrude(vector)`, `revolve(axis, deg)`, `revolve(origin, axis, deg)`, `sweep(path)`, `fillet(r)`, `fillet(r, edges)`, `chamfer(d)`, `chamfer(d, edges)`, `shell(t)`, `shell(t, open_faces)`, `offset(d)` (2D), `clean()`, `to_face()` |
| exploration | `faces()`, `edges()`, `vertices()`, `wires()`, `shells()`, `solids()`, `faces(sel)`, `edges(sel)`, `vertices(sel)`, `face(sel)`, `edge(sel)`, `vertex(sel)`, `top_face()`, `bottom_face()`, `children()` |
| measurement | `volume()`, `area()`, `length()`, `center()`, `bounding_box()`, `distance(other)`, `closest_points(other)`, `is_valid()`, `is_closed()`, `shape_type()`, `is_same(other)`, `==` |
| edges, faces, vertices | `point()`, `start_point()`, `end_point()`, `point_at(t)`, `tangent_at(t)`, `radius()`, `geom_type()`, `normal()`, `plane()` (a sketch plane on a planar face), `outer_wire()` |
| output | `mesh(tolerance)`, `save(path)` (by extension), `export_step(path)`, `export_iges(path)`, `export_brep(path)`, `export_stl(path)`, `export_stl(path, tolerance)` |

### Selections and selectors

`faces()`, `edges()` and friends return a `Selection`: an ordered list of shapes that supports
`len()`, indexing (`sel[0]`, `sel[-1]`), `first()`, `last()`, `single()`, `for x in sel { }`,
`filter(|f| ...)`, `sort_by(|f| key)`, `farthest(dir)`, `nearest(dir)`, `top()`, `bottom()`,
`left()`, `right()`, `front()`, `back()`, `parallel_to(dir)`, `perpendicular_to(dir)`,
`facing(dir)`, `of_type(name)`, `union`/`+`, `intersect`, `subtract`/`-`, `to_array()` and
`compound()`.

`select(selector)` (and every `sel` argument above) takes a CadQuery-style selector string:

| selector | keeps |
|----------|-------|
| `>Z` / `<Z` | shapes whose centers are farthest / nearest along +Z (ties within 1e-4 are kept) |
| `>Z[1]` | the second level along +Z; `[-1]` is the last level |
| `\|Z` | planar faces whose normal, or straight edges whose direction, is parallel to Z |
| `#Z` | ... perpendicular to Z |
| `+Z` / `-Z` | ... pointing along +Z / -Z |
| `%CIRCLE` | edges or faces of a geometry type (`LINE`, `CIRCLE`, `BSPLINE`, `PLANE`, `CYLINDER`, `SPHERE`, ...) |

Directions are `X`, `Y`, `Z` or vectors like `(1, 1, 0)`. Combine selectors with `and`, `or`,
`not`, `exc` and parentheses: `box.edges("|Z and >X")`, `faces("not (%PLANE or %CYLINDER)")`.

### Sketches

A `Sketch` draws lines and arcs in a plane's 2D coordinates and turns them into wires or faces:

```rhai
let profile = sketch("XZ")
    .move_to(0, 0)
    .hline(10)
    .vline(2)
    .arc_to(5, 6, 0, 2)  // through (5, 6), ending at (0, 2)
    .close();             // closed wire; .face() for a face, .wire() for an open wire
profile.translate([20, 0, 0]).revolve("Z", 360)
```

Pen moves: `move_to(x, y)` (before drawing), `line_to(x, y)`, `line(dx, dy)`, `hline(dx)`,
`vline(dy)`, `hline_to(x)`, `vline_to(y)`, `polar_line(length, deg)`, `arc_to(mx, my, x, y)`.
Use a face as a sketch plane with `face.plane()`:

```rhai
let block = box(30, 20, 10);
let hole = circle(block.top_face().plane(), 4).extrude(-10);
block - hole
```

More examples live in [`crates/opencascade-rhai/examples`](crates/opencascade-rhai/examples);
the test suite runs every one of them.

### Using it from Rust

The Rhai layer is a thin wrapper over a Rust API with the same shape, which you can use directly:

```rust
use opencascade_rhai::{DVec3, Shape};

fn main() -> Result<(), opencascade_rhai::Error> {
    let plate = Shape::make_box(40.0, 30.0, 5.0)?;
    let vertical_edges = plate.edges().select("|Z")?;
    let hole = Shape::cylinder(4.0, 5.0)?.translate(DVec3::new(20.0, 15.0, 0.0))?;
    let part = plate.fillet(2.0, vertical_edges.shapes())?.cut(&hole)?;
    part.write_step("plate.step")
}
```

## Building

You need a Rust toolchain, CMake and a C++ compiler. By default (the `builtin` feature)
OpenCASCADE is compiled from source by `occt-sys`; this takes roughly 20 minutes on 4 cores and is
cached under `target/OCCT`. To link a system installation instead, build with
`--no-default-features` (set `DEP_OCCT_ROOT` to its install prefix if CMake cannot find it).

## Tests and coverage

All tests use the real OpenCASCADE libraries; nothing is mocked.

```sh
cargo test --workspace
```

* `crates/opencascade-sys/tests` exercise every binding module, including the exception
  translation (each `Standard_Failure` subtype that the bindings document), from multiple threads.
* `crates/opencascade-rhai/tests` cover the Rust API (primitives, operations, selections,
  sketches, meshes, file I/O), every function registered with Rhai through real scripts, the CLI
  (in-process and as a subprocess) and the bundled example scripts.

Coverage (reusing the OpenCASCADE build so it is not compiled a second time):

```sh
cargo install cargo-llvm-cov
DEP_OCCT_ROOT=$PWD/target/OCCT cargo llvm-cov --no-default-features \
    -p opencascade-rhai -p opencascade-sys --summary-only --show-missing-lines
```

CI fails if line coverage drops below 95%. At the time of writing it is about 99% of lines and
functions; the remaining lines are defensive branches for OCCT failure states that validated
inputs cannot reach.

## License

LGPL-2.1, like OpenCASCADE and the upstream `opencascade-rs` crates this builds on.
