pub use inner::*;

#[cxx::bridge]
mod inner {
    unsafe extern "C++" {
        include!("scratchcad-occt-sys/include/shape_analysis.hxx");

        type TopoDS_Shape = crate::topo_ds::TopoDS_Shape;
        type Handle_TopTools_HSequenceOfShape = crate::top_tools::Handle_TopTools_HSequenceOfShape;

        type ShapeAnalysis_FreeBounds;
        #[Self = "ShapeAnalysis_FreeBounds"]
        pub fn ConnectEdgesToWires(
            edges: Pin<&mut Handle_TopTools_HSequenceOfShape>,
            tolerance: f64,
            shared: bool,
            wires: Pin<&mut Handle_TopTools_HSequenceOfShape>,
        ) -> Result<()>;
    }
}
