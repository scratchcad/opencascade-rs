//! Triangle meshes of shapes, for rendering or STL export.

use crate::{
    error::{Error, Result},
    math::{from_pnt, require_positive},
    shape::Shape,
};
use glam::DVec3;
use scratchcad_occt_sys as ffi;
use std::fmt;

/// An indexed triangle mesh. Every three entries of `indices` form one counter-clockwise
/// (outward-facing) triangle; `normals[i]` is the surface normal at `vertices[i]`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mesh {
    pub vertices: Vec<DVec3>,
    pub normals: Vec<DVec3>,
    pub indices: Vec<usize>,
}

impl Mesh {
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// The corners of every triangle.
    pub fn triangles(&self) -> impl Iterator<Item = [DVec3; 3]> + '_ {
        self.indices
            .chunks_exact(3)
            .map(|t| [self.vertices[t[0]], self.vertices[t[1]], self.vertices[t[2]]])
    }

    /// The total area of all triangles.
    pub fn area(&self) -> f64 {
        self.triangles().map(|[a, b, c]| (b - a).cross(c - a).length() / 2.0).sum()
    }
}

impl fmt::Display for Mesh {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Mesh({} vertices, {} triangles)", self.vertex_count(), self.triangle_count())
    }
}

impl Shape {
    /// Triangulates every face. `tolerance` is the maximum distance between the mesh and the
    /// true surface; smaller values produce finer meshes.
    pub fn mesh(&self, tolerance: f64) -> Result<Mesh> {
        require_positive(tolerance, "mesh tolerance")?;
        let mesher = ffi::b_rep_mesh::IncrementalMesh_new(self.raw(), tolerance)?;
        if !mesher.IsDone() {
            return Err(Error::OperationFailed("meshing"));
        }

        let mut mesh = Mesh::default();
        for face in self.faces() {
            let topo_face = face.face()?;
            let reversed =
                topo_face.Orientation() == ffi::top_abs::TopAbs_Orientation::TopAbs_REVERSED;

            let mut location = ffi::top_loc::Location_new();
            let handle = ffi::b_rep::BRep_Tool_Triangulation(topo_face, location.pin_mut())?;
            let Ok(triangulation) = ffi::poly::Handle_Poly_Triangulation_Get(&handle) else {
                return Err(Error::OperationFailed("meshing a face"));
            };
            let transform = ffi::top_loc::TopLoc_Location_Transformation(&location);
            ffi::b_rep_lib::BRepLib_ToolTriangulatedShape::ComputeNormals(topo_face, &handle)?;

            let offset = mesh.vertices.len();
            for i in 1..=triangulation.NbNodes() {
                let mut node = ffi::poly::Poly_Triangulation_Node(triangulation, i)?;
                node.pin_mut().Transform(&transform);
                mesh.vertices.push(from_pnt(&node));

                let mut normal = ffi::poly::Poly_Triangulation_Normal(triangulation, i)?;
                normal.pin_mut().Transform(&transform);
                let normal = DVec3::new(normal.X(), normal.Y(), normal.Z());
                mesh.normals.push(if reversed { -normal } else { normal });
            }

            for i in 1..=triangulation.NbTriangles() {
                let triangle = triangulation.Triangle(i)?;
                let corner =
                    |n: i32| -> Result<usize> { Ok(offset + triangle.Value(n)? as usize - 1) };
                let (a, b, c) = (corner(1)?, corner(2)?, corner(3)?);
                mesh.indices.extend(if reversed { [a, c, b] } else { [a, b, c] });
            }
        }
        Ok(mesh)
    }
}
