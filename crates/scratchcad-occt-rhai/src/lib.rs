//! Script the [OpenCASCADE](https://dev.opencascade.org) CAD kernel with [Rhai](https://rhai.rs).
//!
//! The crate has two layers:
//!
//! * A Rust API built around cheap-to-clone, immutable handle types ([`Shape`], [`Selection`],
//!   [`Sketch`], [`Plane`]) whose operations all return [`Result`] and chain naturally.
//! * [`register`], which exposes that API to a Rhai [`Engine`] with
//!   script-friendly argument conversions.
//!
//! ```no_run
//! let part = scratchcad_occt_rhai::eval_shape(r#"
//!     let plate = box(40, 30, 5).fillet(2, "|Z");
//!     let hole = cylinder(4, 5).translate([20, 15, 0]);
//!     (plate - hole).chamfer(0.5, ">Z")
//! "#)?;
//! part.write_step("plate.step")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! The same model through the Rust API:
//!
//! ```no_run
//! use scratchcad_occt_rhai::{DVec3, Shape};
//!
//! fn main() -> Result<(), scratchcad_occt_rhai::Error> {
//!     let plate = Shape::make_box(40.0, 30.0, 5.0)?;
//!     let vertical_edges = plate.edges().select("|Z")?;
//!     let hole = Shape::cylinder(4.0, 5.0)?.translate(DVec3::new(20.0, 15.0, 0.0))?;
//!     let part = plate.fillet(2.0, vertical_edges.shapes())?.cut(&hole)?;
//!     part.write_step("plate.step")
//! }
//! ```

pub mod cli;
mod error;
mod io;
pub mod math;
mod mesh;
pub mod script;
mod selection;
pub mod selector;
mod shape;
mod sketch;

pub use error::{Error, Result};
pub use glam::{DVec2, DVec3};
pub use math::Plane;
pub use mesh::Mesh;
pub use rhai;
pub use scratchcad_occt_sys as ffi;
pub use script::{register, to_shape};
pub use selection::{Selection, TOLERANCE};
pub use shape::{BoundingBox, Shape, ShapeType};
pub use sketch::Sketch;

use rhai::{Dynamic, Engine, EvalAltResult};

/// Enables or disables OpenCASCADE's own console output (such as the STEP/IGES transfer
/// statistics it prints to stdout). Failures are always printed.
pub fn set_occt_messages(enabled: bool) {
    use ffi::message::{Message_Gravity, Message_set_default_trace_level};
    Message_set_default_trace_level(if enabled {
        Message_Gravity::Message_Info
    } else {
        Message_Gravity::Message_Fail
    });
}

/// A Rhai engine with every type and function of this crate registered.
pub fn engine() -> Engine {
    let mut engine = Engine::new();
    register(&mut engine);
    engine
}

/// Evaluates a script and returns its result.
pub fn eval(script: &str) -> std::result::Result<Dynamic, Box<EvalAltResult>> {
    engine().eval::<Dynamic>(script)
}

/// Evaluates a script that produces a shape (or a selection, array of shapes or sketch, which
/// are converted with [`to_shape`]).
pub fn eval_shape(script: &str) -> std::result::Result<Shape, Box<EvalAltResult>> {
    to_shape(eval(script)?)
}
