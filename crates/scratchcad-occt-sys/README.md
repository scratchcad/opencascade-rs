# scratchcad-occt-sys

Low-level [cxx](https://cxx.rs) bindings to OpenCASCADE 7.8, forked from
[opencascade-rs](https://github.com/bschwind/opencascade-rs)'s `opencascade-sys` for use by
`scratchcad-occt-rhai`.

Differences from upstream:

* **C++ exceptions become `Result`s.** A custom `rust::behavior::trycatch` in
  `include/bindings_common.hxx` translates OCCT's `Standard_Failure` (and `std::exception`) into
  `cxx::Exception` with the message `"<exception type>: <message>"`, and every binding that can
  throw returns `Result`. Upstream, any OCCT exception aborts the process.
* **Explicit checks where OCCT's are compiled out.** OCCT's release build defines `No_Exception`,
  disabling the `*_Raise_if` checks inside its libraries; helpers whose OCCT calls would otherwise
  return null handles or crash check explicitly and throw. Handle dereferences throw
  `Standard_NullObject` instead of dereferencing null.
* **New bindings:** `BRepCheck_Analyzer`, `BRepExtrema_DistShapeShape`, `BRepAdaptor_Surface`
  (`GeomAbs_SurfaceType`), `BRepAdaptor_Curve::{D1, Circle, IsClosed}`, `BRepBndLib::AddOptimal`,
  `BRep_Tool::IsClosed`, plane/point mirroring and composition for `gp_Trsf`, `gp_Ax2` with an
  explicit X direction, `TopoDS_Shape::{IsSame, NbChildren, Closed}`, OCCT message verbosity, and a
  checked `GeomAPI_ProjectPointOnSurf` projection.

OpenCASCADE itself comes from the unmodified [`occt-sys`](https://crates.io/crates/occt-sys) crate
when the `builtin` feature is enabled; otherwise a system installation is located with CMake.
