# AGENTS.md

Guidance for coding agents (and people) working in this repository. `CLAUDE.md` imports this file.

## What this is

Two crates, published to crates.io and consumed by [scratchcad](https://github.com/scratchcad/scratchcad),
an HTTP service that runs untrusted Rhai scripts:

* `crates/scratchcad-occt-sys`: low-level [cxx](https://cxx.rs) bindings to OpenCASCADE (OCCT) 7.8.
* `crates/scratchcad-occt-rhai`: the Rust API (`Shape`, `Selection`, `Sketch`, `Mesh`, ...) and its
  Rhai registration (`register`, `engine()`).

License: LGPL-2.1-only. Files derived from [opencascade-rs](https://github.com/bschwind/opencascade-rs)
start with a "Modified by ... Based on opencascade-rs" header; keep it on those files and do not add it
to new ones.

## OpenCASCADE is prebuilt: do not compile it

The crates link an installed OCCT 7.8 by default. Compiling it from source (the opt-in `builtin`
feature) takes 15 to 20 minutes on four cores, and a cloud session's container can restart mid-build,
killing it.

**Claude Code cloud sessions** are configured to install it: the environment's setup script installs
conda-forge's `occt=7.8.1` into `/opt/occt`, and the environment sets `DEP_OCCT_ROOT=/opt/occt` (where
CMake finds it) and `LD_LIBRARY_PATH=/opt/occt/lib` (where test binaries find the shared libraries).
Check with `ls $DEP_OCCT_ROOT/lib/libTKernel.so*`. If it is missing (another environment, or the setup
changed), install it yourself; it takes about 20 seconds:

```sh
curl -fsSL https://micro.mamba.pm/api/micromamba/linux-64/latest | tar -xj -C /usr/local bin/micromamba
MAMBA_ROOT_PREFIX=/opt/mamba micromamba create -y -p /opt/occt -c conda-forge "occt=7.8.1=novtk*"
export DEP_OCCT_ROOT=/opt/occt LD_LIBRARY_PATH=/opt/occt/lib
```

A clean build of the workspace plus all tests then takes about two minutes. Do not build with
`--features builtin`: nothing (CI included) builds OpenCASCADE from source any more, and the feature
is kept only for people without a prebuilt OCCT.

Elsewhere: Debian 13 and Ubuntu 25.04+ package OCCT 7.8 (`libocct-*-dev`); Debian 12 and Ubuntu 24.04
only have 7.6, which is too old (7.8 renamed the data-exchange toolkits the bindings link).

## Checks

CI (`.github/workflows`) runs these; run the relevant ones before pushing:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo llvm-cov -p scratchcad-occt-rhai -p scratchcad-occt-sys --summary-only --fail-under-lines 95
clang-format --dry-run --Werror crates/scratchcad-occt-sys/include/*.hxx
```

* Coverage: `rustup component add llvm-tools-preview && cargo install cargo-llvm-cov --locked`. The
  gate is 95% of lines across both crates (currently about 98.5%).
* `clang-format`: CI uses version 14 with `.clang-format` (LLVM style, 120 columns). Include order
  matters.
* `--all-targets` does not run doctests: run `cargo test --workspace --doc` after touching examples in
  doc comments.
* CI installs the prebuilt OCCT with `.github/actions/install-occt`. The `builtin` feature is not
  tested in CI.
* Tests run against the real OCCT; there are no mocks. Shared helpers are in `tests/common/mod.rs`.

## Layout

`scratchcad-occt-sys`:

* One module per OCCT package, in snake case (`BRepAlgoAPI` -> `src/b_rep_algo_api.rs`), each a cxx
  bridge with C++ helpers in `include/<module>.hxx`. `build.rs` compiles every `src/*.rs` except
  `lib.rs` as a bridge and links the toolkits in `OCCT_LIBS`. A new module needs `src/x.rs`,
  `include/x.hxx` and `pub mod x;` in `lib.rs`.
* Every binding that can throw returns `Result`: `include/bindings_common.hxx` turns OCCT's
  `Standard_Failure` (which is not a `std::exception`) into `cxx::Exception`. Dereference handles from
  Rust through `check_handle`/`handle_try_deref`, never directly.
* C++ that calls back into Rust (cancellation, message printing) uses `extern "Rust"` items defined in
  the bridge's parent module and forward-declared in the header; see `src/message.rs`.

`scratchcad-occt-rhai`:

| file | what |
|------|------|
| `shape.rs` | `Shape` and its operations, `DetachedShape`, point classification |
| `io.rs` | STEP/IGES/BREP/STL to and from bytes and files, `Format` |
| `mesh.rs` | `Mesh`, `MeshOptions`, welding, STL writers |
| `cancel.rs` | `CancelToken`, `with_cancel`, the progress ranges algorithms poll |
| `messages.rs` | where OCCT's console messages go |
| `error.rs` | `Error`, `ErrorKind` (carried on script errors as the Rhai tag) |
| `script.rs` | the Rhai registration; `docs/rhai-api.md` documents it |
| `selection.rs`, `selector.rs`, `sketch.rs`, `math.rs` | selections and selector strings, sketches, vectors and planes |

Update `docs/rhai-api.md` (and the README when it is user-facing) with any change to what scripts can
call.

## Design rules

These are deliberate; keep them.

1. **Scripts cannot touch the filesystem.** Nothing `register` adds reads or writes files: the host
   does that through the Rust API (`Shape::to_bytes`/`from_bytes`, `export`/`import`) and passes shapes
   to scripts in their `Scope`. `tests/script.rs` (`scripts_cannot_touch_files`) guards this. Rhai's
   `disable_symbol` cannot hide functions, so never register a file function at all.
2. **`Shape` is neither `Send` nor `Sync`** (it wraps an `Rc`). OCCT shapes share sub-shapes, and many
   OCCT algorithms update their inputs' sub-shapes in place (new parameter curves, larger tolerances),
   so sharing shapes between threads would race. Do not switch to `Arc` or add `unsafe impl Sync`;
   `Shape::detach` makes a deep copy that is `Send`. Rhai's `sync` feature cannot be enabled for the
   same reason.
3. **Operations leave their inputs alone where OCCT allows it.** Booleans run in OCCT's
   non-destructive mode (`run_boolean` in `include/b_rep_algo_api.hxx`); meshing works on a
   `deep_copy`, which also stops OCCT from reusing an older, finer triangulation.
4. **Cancellation.** Give every algorithm that takes a `Message_ProgressRange` a fresh
   `cancel::progress()` (a range can only be used once), report a failed algorithm with
   `cancel::failed("name")` (it becomes `Error::Cancelled` when the token was cancelled), and call
   `cancel::check()` before work OCCT cannot interrupt. In OCCT 7.8, booleans, shells, meshing (between
   faces) and STEP/IGES/BREP conversion poll the range; fillets, chamfers and lofts ignore it, and
   sweeps and `clean` take none. `tests/cancel.rs` checks each.
5. **STEP and IGES conversion holds `io::translator`'s lock**: OCCT's translators keep global state.
   BREP and STL do not need it.
6. **OCCT's console output.** OCCT does not synchronize its printer list, so `messages.rs` swaps the
   printer once and routes messages in Rust afterwards. Do not call `Message_use_*_printer` anywhere
   else.
7. **Errors.** Every fallible function returns the crate's `Result`. Keep error messages stable and
   specific: scratchcad passes them on to a language model that fixes the script.

## OCCT and Rhai details learned the hard way

* `BRepTools::Write` to a stream omits the `DBRep_DrawableShape` line that the file writer adds;
  `write_brep_bytes` adds it, so in-memory and file BREP are identical.
* OCCT 7.8 cannot read IGES from a stream; `Shape::from_iges` goes through an exclusively created
  temporary file.
* OCCT 7.8's data-exchange toolkits are `TKDESTEP`, `TKDEIGES`, `TKDESTL`; 7.6 names them differently.
* Rhai's `Engine::gen_fn_signatures` needs Rhai's `metadata` feature, which is off; test the presence
  of a function by calling it.

## Releasing

Publish `scratchcad-occt-sys` before `scratchcad-occt-rhai`, which depends on it by version and path,
and bump the minor version (0.x) for breaking changes. scratchcad pins an exact version
(`scratchcad-occt-rhai = "=x.y.z"`), so it moves to a new release deliberately.
