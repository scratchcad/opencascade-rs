mod common;

use common::{assert_close, cube10, err_contains};
use opencascade_rhai::{Mesh, Shape};
use std::f64::consts::PI;

#[test]
fn box_mesh() {
    let cube = cube10();
    let mesh = cube.mesh(0.1).unwrap();
    assert_eq!(mesh.triangle_count(), 12);
    assert_eq!(mesh.vertex_count(), 24, "each face has its own 4 corners");
    assert_eq!(mesh.normals.len(), mesh.vertices.len());
    assert_close(mesh.area(), 600.0, 1e-9);
    assert_eq!(mesh.triangles().count(), 12);
    assert_eq!(mesh.to_string(), "Mesh(24 vertices, 12 triangles)");

    // Normals and winding both point outwards.
    let center = cube.center().unwrap();
    for (vertex, normal) in mesh.vertices.iter().zip(&mesh.normals) {
        assert!((*vertex - center).dot(*normal) > 0.0, "inward normal {normal} at {vertex}");
    }
    for [a, b, c] in mesh.triangles() {
        let winding = (b - a).cross(c - a);
        assert!(winding.dot((a + b + c) / 3.0 - center) > 0.0);
    }
}

#[test]
fn curved_meshes_converge() {
    let sphere = Shape::sphere(2.0).unwrap();
    let coarse = sphere.mesh(0.1).unwrap();
    let fine = sphere.mesh(0.001).unwrap();
    assert!(fine.triangle_count() > coarse.triangle_count());
    let exact = 16.0 * PI;
    assert!((fine.area() - exact).abs() < (coarse.area() - exact).abs());
    assert_close(fine.area(), exact, 0.05);
    for (vertex, normal) in fine.vertices.iter().zip(&fine.normals) {
        assert!(vertex.dot(*normal) > 0.0);
    }
    // Reversed faces (from a mirror) still yield outward normals.
    let mirrored = Shape::cylinder(1.0, 1.0).unwrap().scale(glam::DVec3::ZERO, -1.0).unwrap();
    let mesh = mirrored.mesh(0.01).unwrap();
    let center = mirrored.center().unwrap();
    let outward = mesh
        .vertices
        .iter()
        .zip(&mesh.normals)
        .filter(|(p, n)| (**p - center).dot(**n) > 0.0)
        .count();
    assert_eq!(outward, mesh.vertices.len());
}

#[test]
fn invalid_tolerances_and_empty_shapes() {
    err_contains(cube10().mesh(0.0), "mesh tolerance");
    err_contains(cube10().mesh(f64::NAN), "mesh tolerance");
    let empty = Shape::empty().unwrap().mesh(0.1).unwrap();
    assert_eq!(empty, Mesh::default());
    assert_eq!(empty.triangle_count(), 0);
    assert_eq!(empty.area(), 0.0);
}
