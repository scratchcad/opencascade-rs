# AGENTS.md

Notes for coding agents; `CLAUDE.md` imports this file. Read [README.md](README.md) (usage, building),
[docs/rhai-api.md](docs/rhai-api.md) (what scripts can call) and
[crates/scratchcad-occt-sys/README.md](crates/scratchcad-occt-sys/README.md) (how the bindings work)
first. The crates are consumed by [scratchcad](https://github.com/scratchcad/scratchcad), an HTTP
service that runs untrusted scripts.

## Environment

Never compile OpenCASCADE (no `--features builtin`): it takes 20 minutes and cloud containers can
restart mid-build. Cloud sessions install conda-forge's prebuilt OCCT 7.8.1 into `/opt/occt` and set
`DEP_OCCT_ROOT` and `LD_LIBRARY_PATH`. If `ls $DEP_OCCT_ROOT/lib/libTKernel.so*` finds nothing,
install it (about 20 seconds):

```sh
curl -fsSL https://micro.mamba.pm/api/micromamba/linux-64/latest | tar -xj -C /usr/local bin/micromamba
MAMBA_ROOT_PREFIX=/opt/mamba micromamba create -y -p /opt/occt -c conda-forge "occt=7.8.1=novtk*"
export DEP_OCCT_ROOT=/opt/occt LD_LIBRARY_PATH=/opt/occt/lib
```

## Checks

CI runs these; run them before pushing (`cargo test --workspace --doc` too after editing doc
examples):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo llvm-cov -p scratchcad-occt-rhai -p scratchcad-occt-sys --summary-only --fail-under-lines 95
clang-format --dry-run --Werror crates/scratchcad-occt-sys/include/*.hxx   # version 14
```

Tests run against the real OCCT; shared helpers are in `tests/common/mod.rs`.

## Conventions

* A new sys module needs `src/x.rs` (a cxx bridge), `include/x.hxx` and `pub mod x;` in `lib.rs`.
  Bindings that can throw return `Result`; dereference handles only through `check_handle` or
  `handle_try_deref`.
* Update `docs/rhai-api.md` with any change to what scripts can call.
* Keep the "Based on opencascade-rs" header on files derived from it; don't add it to new files.
* Releasing: publish `scratchcad-occt-sys` before `scratchcad-occt-rhai`, and bump the minor version
  for breaking changes. scratchcad pins an exact version.

## Design rules

These are deliberate; keep them.

1. **Scripts cannot touch the filesystem.** `register` adds no file functions and sets
   `DummyModuleResolver` so `import` loads nothing. Guarded by `tests/script.rs`.
2. **Scripts cannot forge error kinds.** Errors carry their `ErrorKind` as the Rhai tag, and
   `register` overrides Rhai's tag setters.
3. **`Shape` is not `Send` or `Sync`.** OCCT updates shared sub-shapes in place. Never use `Arc`,
   `unsafe impl Sync` or Rhai's `sync` feature; `Shape::detach` moves a deep copy between threads.
4. **Leave inputs alone.** Booleans run non-destructively; meshing works on a `deep_copy`.
5. **Cancellation.** Give each algorithm a fresh `cancel::progress()`, report failure with
   `cancel::failed("name")`, call `cancel::check()` before work OCCT cannot interrupt, and end
   conversions with `finished` (a stopped translator can return part of a model). Covered by
   `tests/cancel.rs`.
6. **Process-wide OCCT state.** STEP and IGES conversion holds `io::translator`'s lock. Only
   `messages.rs` may swap OCCT's printer.
7. **Errors** go through the crate's `Result`, and their messages stay stable and specific: scratchcad
   passes them to a language model that fixes the script.

## Gotchas

* OCCT 7.8 cannot read IGES from memory, so IGES is read only from host-named files. Never write a
  temporary file to work around this.
* Rhai's `disable_symbol` cannot hide functions, and `gen_fn_signatures` needs the `metadata`
  feature (off): test for a function by calling it.
