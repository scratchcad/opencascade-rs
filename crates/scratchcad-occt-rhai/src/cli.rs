//! The `occt-rhai` command line tool: runs a script and exports the shape it produces.

use crate::to_shape;
use std::{io::Write, path::PathBuf};

/// The STL meshing tolerance used unless `--tolerance` is given.
pub const DEFAULT_STL_TOLERANCE: f64 = 0.01;

pub const USAGE: &str = "\
usage: occt-rhai <script.rhai> [--output <file>] [--tolerance <mm>] [--verbose]

Runs a Rhai script whose last expression is a shape and prints a summary of it.

options:
  -o, --output <file>     write the shape to <file> (.step, .stp, .iges, .igs, .brep, .brp, .stl)
  -t, --tolerance <mm>    STL meshing tolerance (default 0.01)
  -v, --verbose           show OpenCASCADE's own messages (e.g. STEP transfer statistics)
  -h, --help              show this help";

/// Parsed command line arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    pub script: PathBuf,
    pub output: Option<PathBuf>,
    pub tolerance: f64,
    pub verbose: bool,
}

/// What the command line asks for.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Help,
    Run(Options),
}

/// Parses arguments (without the program name).
pub fn parse_args(args: &[String]) -> Result<Command, String> {
    let mut script = None;
    let mut output = None;
    let mut tolerance = DEFAULT_STL_TOLERANCE;
    let mut verbose = false;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "-v" | "--verbose" => verbose = true,
            "-o" | "--output" => {
                let value = iter.next().ok_or_else(|| format!("{arg} needs a file name"))?;
                output = Some(PathBuf::from(value));
            },
            "-t" | "--tolerance" => {
                let value = iter.next().ok_or_else(|| format!("{arg} needs a value"))?;
                tolerance = value
                    .parse::<f64>()
                    .ok()
                    .filter(|t| t.is_finite() && *t > 0.0)
                    .ok_or_else(|| format!("invalid tolerance {value:?}"))?;
            },
            flag if flag.starts_with('-') => return Err(format!("unknown option {flag}")),
            path => {
                if script.replace(PathBuf::from(path)).is_some() {
                    return Err("only one script can be given".into());
                }
            },
        }
    }
    let script = script.ok_or("no script given")?;
    Ok(Command::Run(Options { script, output, tolerance, verbose }))
}

/// Runs the tool, writing human-readable output to `out`.
pub fn run(args: &[String], out: &mut impl Write) -> Result<(), String> {
    let options = match parse_args(args).map_err(|e| format!("{e}\n\n{USAGE}"))? {
        Command::Help => {
            writeln!(out, "{USAGE}").map_err(|e| e.to_string())?;
            return Ok(());
        },
        Command::Run(options) => options,
    };

    crate::set_occt_messages(options.verbose);
    let source = std::fs::read_to_string(&options.script)
        .map_err(|e| format!("cannot read {}: {e}", options.script.display()))?;
    let value = crate::engine()
        .eval::<rhai::Dynamic>(&source)
        .map_err(|e| format!("{}: {e}", options.script.display()))?;
    let shape = to_shape(value).map_err(|e| format!("{}: {e}", options.script.display()))?;

    let report = |out: &mut dyn Write| -> std::io::Result<()> {
        writeln!(out, "{shape}")?;
        if let Ok(bbox) = shape.bounding_box() {
            writeln!(out, "{bbox}")?;
        }
        if let Ok(volume) = shape.volume() {
            writeln!(out, "volume: {volume:.6}")?;
        }
        Ok(())
    };
    report(out).map_err(|e| e.to_string())?;

    if let Some(path) = &options.output {
        shape.export(path, options.tolerance).map_err(|e| e.to_string())?;
        writeln!(out, "wrote {}", path.display()).map_err(|e| e.to_string())?;
    }
    Ok(())
}
