# Rhai API reference

`scratchcad_occt_rhai::engine()` returns a Rhai `Engine` with everything registered;
`scratchcad_occt_rhai::register(&mut engine)` adds it to an engine of your own. `eval_shape(script)`
runs a script and converts its result to a `Shape`.

## Conventions

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

## Constructors

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

## Shape methods

| category | methods |
|----------|---------|
| booleans | `fuse`/`union`/`+`, `cut`/`-`, `common`/`intersect`/`&`, `section` |
| transforms | `translate(v)`, `translate(x, y, z)`, `rotate(axis, deg)`, `rotate(origin, axis, deg)`, `rotate_x/y/z(deg)`, `scale(f)`, `scale(center, f)`, `mirror(plane_or_normal)`, `mirror(origin, normal)` |
| features | `extrude(distance)` (along the face normal), `extrude(vector)`, `revolve(axis, deg)`, `revolve(origin, axis, deg)`, `sweep(path)`, `fillet(r)`, `fillet(r, edges)`, `chamfer(d)`, `chamfer(d, edges)`, `shell(t)`, `shell(t, open_faces)`, `offset(d)` (2D), `clean()`, `to_face()` |
| exploration | `faces()`, `edges()`, `vertices()`, `wires()`, `shells()`, `solids()`, `faces(sel)`, `edges(sel)`, `vertices(sel)`, `face(sel)`, `edge(sel)`, `vertex(sel)`, `top_face()`, `bottom_face()`, `children()` |
| measurement | `volume()`, `area()`, `length()`, `center()`, `bounding_box()`, `distance(other)`, `closest_points(other)`, `is_valid()`, `is_closed()`, `shape_type()`, `is_same(other)`, `==` |
| edges, faces, vertices | `point()`, `start_point()`, `end_point()`, `point_at(t)`, `tangent_at(t)`, `radius()`, `geom_type()`, `normal()`, `plane()` (a sketch plane on a planar face), `outer_wire()` |
| output | `mesh(tolerance)`, `save(path)` (by extension), `export_step(path)`, `export_iges(path)`, `export_brep(path)`, `export_stl(path)`, `export_stl(path, tolerance)` |

## Selections and selectors

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

## Sketches

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

More examples live in [`crates/scratchcad-occt-rhai/examples`](../crates/scratchcad-occt-rhai/examples);
the test suite runs every one of them.
