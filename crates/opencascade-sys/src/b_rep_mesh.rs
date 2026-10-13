pub use inner::*;

#[cxx::bridge]
mod inner {
    unsafe extern "C++" {
        include!("opencascade-sys/include/b_rep_mesh.hxx");

        type TopoDS_Shape = crate::topo_ds::TopoDS_Shape;

        type BRepMesh_IncrementalMesh;
        #[cxx_name = "construct_unique"]
        pub fn IncrementalMesh_new(
            shape: &TopoDS_Shape,
            deflection: f64,
        ) -> Result<UniquePtr<BRepMesh_IncrementalMesh>>;
        pub fn Shape(self: &BRepMesh_IncrementalMesh) -> Result<&TopoDS_Shape>;
        pub fn IsDone(self: &BRepMesh_IncrementalMesh) -> bool;
    }
}
