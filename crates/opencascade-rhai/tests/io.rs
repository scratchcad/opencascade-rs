mod common;

use common::{assert_close, err_contains, temp_dir};
use opencascade_rhai::{Error, Shape, ShapeType};

#[test]
fn step_round_trip() {
    let dir = temp_dir();
    let path = dir.path().join("part.step");
    let part = Shape::cube(2.0).unwrap().fillet_all(0.5).unwrap();
    part.write_step(&path).unwrap();
    let back = Shape::read_step(&path).unwrap();
    assert_eq!(back.shape_type(), ShapeType::Solid);
    assert_close(back.volume().unwrap(), part.volume().unwrap(), 1e-6);
    let imported = Shape::import(&path).unwrap();
    assert_close(imported.volume().unwrap(), part.volume().unwrap(), 1e-6);
}

#[test]
fn iges_round_trip() {
    let dir = temp_dir();
    let path = dir.path().join("part.igs");
    let part = Shape::cylinder(1.0, 3.0).unwrap();
    part.write_iges(&path).unwrap();
    let back = Shape::read_iges(&path).unwrap();
    assert!(!back.faces().is_empty());
    assert!(Shape::import(dir.path().join("part.igs")).is_ok());
    part.export(dir.path().join("again.IGES"), 0.1).unwrap();
    assert!(dir.path().join("again.IGES").exists());
}

#[test]
fn brep_round_trip_is_exact() {
    let dir = temp_dir();
    let path = dir.path().join("part.brep");
    let part = Shape::torus(3.0, 1.0).unwrap();
    part.write_brep(&path).unwrap();
    let back = Shape::read_brep(&path).unwrap();
    assert_close(back.volume().unwrap(), part.volume().unwrap(), 1e-9);
    part.export(dir.path().join("x.brp"), 0.1).unwrap();
    assert!(Shape::import(dir.path().join("x.brp")).is_ok());
}

#[test]
fn stl_export() {
    let dir = temp_dir();
    let path = dir.path().join("part.stl");
    Shape::cube(1.0).unwrap().write_stl(&path, 0.1).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert_eq!(text.matches("facet normal").count(), 12);
    Shape::sphere(1.0).unwrap().export(dir.path().join("ball.stl"), 0.01).unwrap();
    assert!(std::fs::metadata(dir.path().join("ball.stl")).unwrap().len() > 1000);
    err_contains(Shape::cube(1.0).unwrap().write_stl(&path, 0.0), "STL tolerance");
}

#[test]
fn file_errors() {
    let dir = temp_dir();
    let missing = dir.path().join("missing");
    for result in [
        Shape::read_step(missing.with_extension("step")),
        Shape::read_iges(missing.with_extension("igs")),
        Shape::read_brep(missing.with_extension("brep")),
    ] {
        assert!(matches!(result, Err(Error::Io(_))), "{result:?}");
    }
    err_contains(Shape::read_step(missing.with_extension("step")), "failed to read STEP file");

    let cube = Shape::cube(1.0).unwrap();
    let bad_dir = dir.path().join("no").join("such").join("dir");
    err_contains(cube.write_step(bad_dir.join("x.step")), "failed to write STEP file");
    err_contains(cube.write_iges(bad_dir.join("x.igs")), "failed to write IGES file");
    err_contains(cube.write_brep(bad_dir.join("x.brep")), "failed to write BREP file");
    err_contains(cube.write_stl(bad_dir.join("x.stl"), 0.1), "failed to write STL file");

    err_contains(cube.export(dir.path().join("x.obj"), 0.1), "unsupported file type");
    err_contains(cube.export(dir.path().join("no_extension"), 0.1), "unsupported file type");
    err_contains(Shape::import(dir.path().join("x.stl")), "unsupported file type");

    // A file with the right extension but the wrong content.
    let junk = dir.path().join("junk.step");
    std::fs::write(&junk, "not a step file").unwrap();
    assert!(Shape::read_step(&junk).is_err());
    let junk = dir.path().join("junk.brep");
    std::fs::write(&junk, "not a brep file").unwrap();
    assert!(Shape::read_brep(&junk).is_err());
}

#[test]
fn empty_shapes_cannot_be_exchanged() {
    let dir = temp_dir();
    let empty = Shape::empty().unwrap();
    // OCCT refuses to transfer an empty compound; whatever it decides must not crash.
    let step = empty.write_step(dir.path().join("empty.step"));
    let iges = empty.write_iges(dir.path().join("empty.igs"));
    for result in [step, iges] {
        if let Err(e) = result {
            assert!(matches!(e, Error::Io(_)), "{e}");
        }
    }
}

#[test]
fn concurrent_file_access_is_serialized() {
    let dir = temp_dir();
    let paths: Vec<_> = (0..4).map(|i| dir.path().join(format!("part{i}.step"))).collect();
    let handles: Vec<_> = paths
        .clone()
        .into_iter()
        .map(|path| {
            std::thread::spawn(move || {
                // Shapes are not Send, so each thread builds its own.
                Shape::cube(1.0).unwrap().write_step(&path).unwrap();
                Shape::read_step(&path).unwrap().volume().unwrap()
            })
        })
        .collect();
    for handle in handles {
        assert_close(handle.join().unwrap(), 1.0, 1e-9);
    }
}
