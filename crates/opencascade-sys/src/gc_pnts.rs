pub use inner::*;

#[cxx::bridge]
mod inner {
    unsafe extern "C++" {
        include!("opencascade-sys/include/gc_pnts.hxx");

        type gp_Pnt = crate::gp::gp_Pnt;
        type BRepAdaptor_Curve = crate::b_rep_adaptor::BRepAdaptor_Curve;

        type GCPnts_TangentialDeflection;
        #[cxx_name = "construct_unique"]
        pub fn TangentialDeflection_new(
            curve: &BRepAdaptor_Curve,
            angular_deflection: f64,
            curvature_deflection: f64,
        ) -> Result<UniquePtr<GCPnts_TangentialDeflection>>;
        pub fn NbPoints(self: &GCPnts_TangentialDeflection) -> i32;
        pub fn GCPnts_TangentialDeflection_Value(
            approximator: &GCPnts_TangentialDeflection,
            index: i32,
        ) -> Result<UniquePtr<gp_Pnt>>;
    }
}
