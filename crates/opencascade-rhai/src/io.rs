//! Reading and writing STEP, IGES, BREP and STL files.
//!
//! OpenCASCADE's data exchange code keeps global state, so all file operations are serialized
//! through a process-wide lock.

use crate::{
    error::{Error, Result},
    math::require_positive,
    shape::Shape,
};
use opencascade_sys as ffi;
use std::{path::Path, sync::Mutex};

static IO_LOCK: Mutex<()> = Mutex::new(());

fn locked<T>(f: impl FnOnce() -> Result<T>) -> Result<T> {
    // A panic while holding the lock cannot leave OCCT in a worse state than the panic itself.
    let _guard = IO_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    f()
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn io_error(action: &str, format: &str, path: &Path) -> Error {
    Error::Io(format!("failed to {action} {format} file {}", path.display()))
}

fn progress() -> cxx::UniquePtr<ffi::message::Message_ProgressRange> {
    ffi::message::Message_ProgressRange_new()
}

impl Shape {
    /// Reads all shapes from a STEP file (as a compound when there are several).
    pub fn read_step(path: impl AsRef<Path>) -> Result<Shape> {
        let path = path.as_ref();
        locked(|| {
            let mut reader = ffi::step_control::STEPControl_Reader_new();
            let status = ffi::step_control::read_step(reader.pin_mut(), path_string(path))?;
            if status != ffi::if_select::IFSelect_ReturnStatus::IFSelect_RetDone {
                return Err(io_error("read", "STEP", path));
            }
            reader.pin_mut().TransferRoots(&progress())?;
            Shape::from_unique(ffi::step_control::one_shape_step(&reader)?)
                .map_err(|_| io_error("read", "STEP", path))
        })
    }

    pub fn write_step(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        locked(|| {
            let mut writer = ffi::step_control::STEPControl_Writer_new();
            let status = ffi::step_control::transfer_shape(writer.pin_mut(), self.raw())?;
            if status != ffi::if_select::IFSelect_ReturnStatus::IFSelect_RetDone {
                return Err(io_error("convert shape for", "STEP", path));
            }
            let status = ffi::step_control::write_step(writer.pin_mut(), path_string(path))?;
            if status != ffi::if_select::IFSelect_ReturnStatus::IFSelect_RetDone {
                return Err(io_error("write", "STEP", path));
            }
            Ok(())
        })
    }

    /// Reads all shapes from an IGES file (as a compound when there are several).
    pub fn read_iges(path: impl AsRef<Path>) -> Result<Shape> {
        let path = path.as_ref();
        locked(|| {
            let mut reader = ffi::iges_control::IGESControl_Reader_new();
            let status = ffi::iges_control::read_iges(reader.pin_mut(), path_string(path))?;
            if status != ffi::if_select::IFSelect_ReturnStatus::IFSelect_RetDone {
                return Err(io_error("read", "IGES", path));
            }
            reader.pin_mut().TransferRoots(&progress())?;
            Shape::from_unique(ffi::iges_control::one_shape_iges(&reader)?)
                .map_err(|_| io_error("read", "IGES", path))
        })
    }

    pub fn write_iges(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        locked(|| {
            let mut writer = ffi::iges_control::IGESControl_Writer_new();
            if !writer.pin_mut().AddShape(self.raw(), &progress())? {
                return Err(io_error("convert shape for", "IGES", path));
            }
            writer.pin_mut().ComputeModel()?;
            if !ffi::iges_control::write_iges(writer.pin_mut(), path_string(path))? {
                return Err(io_error("write", "IGES", path));
            }
            Ok(())
        })
    }

    /// Reads OpenCASCADE's native text BREP format.
    pub fn read_brep(path: impl AsRef<Path>) -> Result<Shape> {
        let path = path.as_ref();
        locked(|| {
            Shape::from_unique(ffi::b_rep_tools::read(path_string(path))?)
                .map_err(|_| io_error("read", "BREP", path))
        })
    }

    /// Writes OpenCASCADE's native text BREP format, which round-trips shapes exactly.
    pub fn write_brep(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        locked(|| {
            if ffi::b_rep_tools::write(self.raw(), path_string(path))? {
                Ok(())
            } else {
                Err(io_error("write", "BREP", path))
            }
        })
    }

    /// Meshes the shape with the given tolerance and writes a binary STL file.
    pub fn write_stl(&self, path: impl AsRef<Path>, tolerance: f64) -> Result<()> {
        let path = path.as_ref();
        require_positive(tolerance, "STL tolerance")?;
        locked(|| {
            let mesher = ffi::b_rep_mesh::IncrementalMesh_new(self.raw(), tolerance)?;
            if !mesher.IsDone() {
                return Err(Error::OperationFailed("meshing"));
            }
            let mut writer = ffi::stl_api::StlAPI_Writer_new();
            if ffi::stl_api::write_stl(writer.pin_mut(), self.raw(), path_string(path))? {
                Ok(())
            } else {
                Err(io_error("write", "STL", path))
            }
        })
    }

    /// Writes the shape in the format implied by the file extension: `.step`/`.stp`,
    /// `.iges`/`.igs`, `.brep`/`.brp` or `.stl` (meshed with `stl_tolerance`).
    pub fn export(&self, path: impl AsRef<Path>, stl_tolerance: f64) -> Result<()> {
        let path = path.as_ref();
        match extension(path).as_deref() {
            Some("step" | "stp") => self.write_step(path),
            Some("iges" | "igs") => self.write_iges(path),
            Some("brep" | "brp") => self.write_brep(path),
            Some("stl") => self.write_stl(path, stl_tolerance),
            _ => Err(unsupported(path)),
        }
    }

    /// Reads a shape in the format implied by the file extension (STEP, IGES or BREP).
    pub fn import(path: impl AsRef<Path>) -> Result<Shape> {
        let path = path.as_ref();
        match extension(path).as_deref() {
            Some("step" | "stp") => Shape::read_step(path),
            Some("iges" | "igs") => Shape::read_iges(path),
            Some("brep" | "brp") => Shape::read_brep(path),
            _ => Err(unsupported(path)),
        }
    }
}

fn extension(path: &Path) -> Option<String> {
    path.extension().map(|ext| ext.to_string_lossy().to_ascii_lowercase())
}

fn unsupported(path: &Path) -> Error {
    Error::Io(format!(
        "unsupported file type {} (expected .step, .stp, .iges, .igs, .brep, .brp or .stl)",
        path.display()
    ))
}
