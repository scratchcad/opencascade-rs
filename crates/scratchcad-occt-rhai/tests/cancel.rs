//! Stopping OpenCASCADE algorithms through a `CancelToken`.

mod common;

use common::{assert_close, cube10};
use scratchcad_occt_rhai::{
    engine, error_kind, with_cancel, CancelToken, Error, ErrorKind, Format, MeshOptions, Shape,
};
use std::time::{Duration, Instant};

fn cancelled() -> CancelToken {
    let token = CancelToken::new();
    token.cancel();
    token
}

#[track_caller]
fn assert_cancelled<T: std::fmt::Debug>(result: Result<T, Error>) {
    assert!(matches!(result, Err(Error::Cancelled)), "expected Cancelled, got {result:?}");
}

#[test]
fn operations_stop_when_already_cancelled() {
    let a = cube10();
    let b = Shape::sphere(6.0).unwrap();
    let token = cancelled();
    with_cancel(&token, || {
        assert_cancelled(a.fuse(&b));
        assert_cancelled(a.cut(&b));
        assert_cancelled(a.common(&b));
        assert_cancelled(a.section(&b));
        assert_cancelled(a.fillet_all(1.0));
        assert_cancelled(a.chamfer_all(1.0));
        assert_cancelled(a.mesh(0.1));
        assert_cancelled(a.to_step());
        assert_cancelled(a.to_brep());
    });
}

#[test]
fn operations_run_normally_under_a_live_token() {
    let token = CancelToken::new();
    let volume = with_cancel(&token, || {
        let part = cube10().cut(&Shape::sphere(6.0).unwrap())?.fillet_all(0.5)?;
        let bytes = part.to_bytes(Format::Step, &MeshOptions::new(0.1))?;
        Shape::from_step(&bytes)?.volume()
    })
    .unwrap();
    assert!(volume > 0.0 && volume < 1000.0);
    assert!(!token.is_cancelled());
}

#[test]
fn cancelling_from_another_thread_stops_meshing() {
    // Meshing a sphere this finely takes several seconds.
    let sphere = Shape::sphere(10.0).unwrap();
    let options = MeshOptions::new(1e-5);
    let token = CancelToken::new();
    let remote = token.clone();
    let canceller = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(200));
        remote.cancel();
    });
    let start = Instant::now();
    let result = with_cancel(&token, || sphere.mesh_with(&options));
    let elapsed = start.elapsed();
    canceller.join().unwrap();
    assert_cancelled(result);
    assert!(elapsed < Duration::from_secs(5), "took {elapsed:?}");
}

#[test]
fn scripts_report_cancellation() {
    let token = cancelled();
    let error =
        with_cancel(&token, || engine().eval::<Shape>("box(10, 10, 10).fillet(1)")).unwrap_err();
    assert_eq!(error_kind(&error), Some(ErrorKind::Cancelled));
    assert!(error.to_string().contains("cancelled"), "{error}");

    // Outside the scope the same script works.
    let volume: f64 = engine().eval("box(10, 10, 10).fillet(1).volume()").unwrap();
    assert!(volume < 1000.0);
}

#[test]
fn tokens_only_apply_inside_their_scope() {
    let token = cancelled();
    with_cancel(&token, || {});
    assert_close(cube10().fuse(&cube10()).unwrap().volume().unwrap(), 1000.0, 1e-6);
}
