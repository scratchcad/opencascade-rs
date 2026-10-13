mod common;

use common::temp_dir;
use opencascade_rhai::{
    cli::{parse_args, run, Command, Options, USAGE},
    Shape,
};
use std::path::PathBuf;

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

fn run_capture(list: &[&str]) -> Result<String, String> {
    let mut out = Vec::new();
    run(&args(list), &mut out).map(|()| String::from_utf8(out).unwrap())
}

#[test]
fn argument_parsing() {
    assert_eq!(parse_args(&args(&["-h"])).unwrap(), Command::Help);
    assert_eq!(parse_args(&args(&["x.rhai", "--help"])).unwrap(), Command::Help);
    assert_eq!(
        parse_args(&args(&["x.rhai"])).unwrap(),
        Command::Run(Options {
            script: PathBuf::from("x.rhai"),
            output: None,
            tolerance: 0.01,
            verbose: false
        })
    );
    assert_eq!(
        parse_args(&args(&["-o", "out.step", "-t", "0.5", "-v", "x.rhai"])).unwrap(),
        Command::Run(Options {
            script: PathBuf::from("x.rhai"),
            output: Some(PathBuf::from("out.step")),
            tolerance: 0.5,
            verbose: true,
        })
    );
    assert!(matches!(
        parse_args(&args(&["--output", "a.stl", "--tolerance", "1", "--verbose", "x.rhai"]))
            .unwrap(),
        Command::Run(Options { verbose: true, .. })
    ));
    for (list, message) in [
        (&[][..], "no script given"),
        (&["a.rhai", "b.rhai"][..], "only one script"),
        (&["--frobnicate"][..], "unknown option --frobnicate"),
        (&["x.rhai", "-o"][..], "-o needs a file name"),
        (&["x.rhai", "--tolerance"][..], "--tolerance needs a value"),
        (&["x.rhai", "-t", "fine"][..], "invalid tolerance \"fine\""),
        (&["x.rhai", "-t", "-1"][..], "invalid tolerance"),
        (&["x.rhai", "-t", "NaN"][..], "invalid tolerance"),
    ] {
        let err = parse_args(&args(list)).unwrap_err();
        assert!(err.contains(message), "{list:?}: {err}");
    }
}

#[test]
fn help_and_usage_errors() {
    assert_eq!(run_capture(&["--help"]).unwrap().trim_end(), USAGE);
    let err = run_capture(&[]).unwrap_err();
    assert!(err.starts_with("no script given") && err.contains("usage: occt-rhai"), "{err}");
}

#[test]
fn running_scripts() {
    let dir = temp_dir();
    let script = dir.path().join("part.rhai");
    std::fs::write(&script, "box(1, 2, 3)").unwrap();
    let output = dir.path().join("part.step");
    let out = run_capture(&[script.to_str().unwrap(), "-o", output.to_str().unwrap()]).unwrap();
    assert!(out.contains("Shape(solid, 6 faces, 12 edges, 8 vertices)"), "{out}");
    assert!(out.contains("BoundingBox(min: (0, 0, 0), max: (1, 2, 3))"), "{out}");
    assert!(out.contains("volume: 6.000000"), "{out}");
    assert!(out.contains("wrote "), "{out}");
    assert!((Shape::read_step(&output).unwrap().volume().unwrap() - 6.0).abs() < 1e-6);

    let stl = dir.path().join("part.stl");
    run_capture(&[script.to_str().unwrap(), "-o", stl.to_str().unwrap(), "-t", "0.5", "-v"])
        .unwrap();
    assert!(stl.exists());

    // A shape without volume or bounding box still gets a summary.
    std::fs::write(&script, "compound([])").unwrap();
    let out = run_capture(&[script.to_str().unwrap()]).unwrap();
    assert!(out.contains("Shape(compound, 0 faces"), "{out}");
    assert!(!out.contains("BoundingBox"), "{out}");
}

#[test]
fn script_failures_are_reported() {
    let dir = temp_dir();
    let missing = dir.path().join("missing.rhai");
    let err = run_capture(&[missing.to_str().unwrap()]).unwrap_err();
    assert!(err.starts_with("cannot read"), "{err}");

    let script = dir.path().join("bad.rhai");
    std::fs::write(&script, "box(0, 1, 1)").unwrap();
    let err = run_capture(&[script.to_str().unwrap()]).unwrap_err();
    assert!(err.contains("box x size must be positive"), "{err}");

    std::fs::write(&script, "42").unwrap();
    let err = run_capture(&[script.to_str().unwrap()]).unwrap_err();
    assert!(err.contains("must evaluate to a shape"), "{err}");

    std::fs::write(&script, "box(1, 1, 1)").unwrap();
    let bad_output = dir.path().join("out.xyz");
    let err =
        run_capture(&[script.to_str().unwrap(), "-o", bad_output.to_str().unwrap()]).unwrap_err();
    assert!(err.contains("unsupported file type"), "{err}");
}

#[test]
fn writer_failures_are_reported() {
    struct Broken;
    impl std::io::Write for Broken {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("broken pipe"))
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    assert_eq!(run(&args(&["--help"]), &mut Broken).unwrap_err(), "broken pipe");

    let dir = temp_dir();
    let script = dir.path().join("part.rhai");
    std::fs::write(&script, "box(1, 1, 1)").unwrap();
    assert_eq!(run(&args(&[script.to_str().unwrap()]), &mut Broken).unwrap_err(), "broken pipe");
}

#[test]
fn the_binary_runs_scripts_and_reports_errors() {
    let dir = temp_dir();
    let script = dir.path().join("part.rhai");
    std::fs::write(&script, "sphere(1)").unwrap();
    let ok =
        std::process::Command::new(env!("CARGO_BIN_EXE_occt-rhai")).arg(&script).output().unwrap();
    assert!(ok.status.success());
    assert!(String::from_utf8_lossy(&ok.stdout).contains("Shape(solid"));

    let failed = std::process::Command::new(env!("CARGO_BIN_EXE_occt-rhai"))
        .arg("--bogus")
        .output()
        .unwrap();
    assert_eq!(failed.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&failed.stderr).starts_with("error: unknown option --bogus"));
}

#[test]
fn bundled_examples_run() {
    let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    let mut count = 0;
    for entry in std::fs::read_dir(&examples).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "rhai") {
            let source = std::fs::read_to_string(&path).unwrap();
            let shape = opencascade_rhai::eval_shape(&source)
                .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            assert!(shape.is_valid(), "{} produced an invalid shape", path.display());
            assert!(shape.volume().unwrap() > 0.0, "{}", path.display());
            count += 1;
        }
    }
    assert!(count >= 4);
}
