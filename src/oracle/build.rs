//! Compiles upstream's C (`units.toml`) from the pinned checkout into a static library and
//! binds its units and headers with bindgen (docs/PLAN.md §7.3). A header's `static inline`
//! functions are reached through wrappers bindgen writes, compiled with the rest. Without the
//! checkout, the crate builds unavailable (`cfg(oracle_unavailable)`): its API and tests are
//! compiled out.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// The environment variable naming the upstream checkout.
const UPSTREAM_DIR_ENV: &str = "OPENSWOS_UPSTREAM_DIR";

/// The checkout's default place, from the workspace's root (`just upstream_fetch`).
const DEFAULT_UPSTREAM_DIR: [&str; 4] = ["target", "cache", "upstream", "OpenSWOS"];

/// The crate's folder, from the workspace's root.
const CRATE_DEPTH: usize = 2;

/// The cfg set when the oracle cannot be built.
const UNAVAILABLE_CFG: &str = "oracle_unavailable";

/// The static library of upstream's objects.
const LIBRARY: &str = "openswos_upstream";

/// The list of units.
const UNITS_FILE: &str = "units.toml";

/// What bindgen's wrappers of a header's `static` functions are called after: `<binding>`
/// then this, in `OUT_DIR` (bindgen adds `.c`).
const WRAPPERS_SUFFIX: &str = "_static";

/// `units.toml`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Units {
    build: Build,
    unit: Vec<Unit>,
    #[serde(default)]
    header: Vec<Header>,
    data: Vec<DataUnit>,
}

/// How upstream's build compiles a unit.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Build {
    flags: Vec<String>,
    optimization: u32,
    defines: Vec<String>,
    include: String,
    include_dirs: Vec<String>,
}

/// A unit the tests call.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Unit {
    path: String,
    functions: Vec<String>,
    variables: Vec<String>,
}

/// A header whose `static inline` functions the tests call: bound, not compiled (its
/// wrappers are).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    path: String,
    functions: Vec<String>,
}

/// A unit of upstream's data.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DataUnit {
    path: String,
}

fn main() {
    println!("cargo::rustc-check-cfg=cfg({UNAVAILABLE_CFG})");
    println!("cargo::rerun-if-changed={UNITS_FILE}");
    println!("cargo::rerun-if-env-changed={UPSTREAM_DIR_ENV}");
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("set by cargo"));
    let units: Units = toml::from_str(
        &fs::read_to_string(manifest.join(UNITS_FILE)).expect("the crate's units.toml"),
    )
    .expect("a valid units.toml");
    let upstream = env::var_os(UPSTREAM_DIR_ENV).map_or_else(
        || {
            let root = manifest
                .ancestors()
                .nth(CRATE_DEPTH)
                .expect("the workspace's root");
            DEFAULT_UPSTREAM_DIR
                .iter()
                .fold(root.to_path_buf(), |path, part| path.join(part))
        },
        PathBuf::from,
    );
    let mut sources: Vec<PathBuf> = units
        .unit
        .iter()
        .map(|unit| &unit.path)
        .chain(units.data.iter().map(|data| &data.path))
        .map(|path| upstream.join(path))
        .collect();
    let headers: Vec<PathBuf> = units
        .header
        .iter()
        .map(|header| upstream.join(&header.path))
        .collect();
    if let Some(missing) = sources
        .iter()
        .chain(&headers)
        .find(|source| !source.is_file())
    {
        // Rerun once the checkout is there.
        println!("cargo::rerun-if-changed={}", upstream.display());
        println!(
            "cargo::warning=the oracle is unavailable: no {} (just upstream_fetch)",
            missing.display()
        );
        println!("cargo::rustc-cfg={UNAVAILABLE_CFG}");
        return;
    }
    for source in sources.iter().chain(&headers) {
        println!("cargo::rerun-if-changed={}", source.display());
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("set by cargo"));
    for unit in &units.unit {
        let binding = Binding {
            path: &unit.path,
            functions: &unit.functions,
            variables: &unit.variables,
            wrapped: false,
        };
        bind(&upstream, &units.build, &binding, &out);
    }
    for header in &units.header {
        let binding = Binding {
            path: &header.path,
            functions: &header.functions,
            variables: &[],
            wrapped: true,
        };
        let wrappers = bind(&upstream, &units.build, &binding, &out)
            .unwrap_or_else(|| panic!("no static function of {} wrapped", header.path));
        sources.push(wrappers);
    }
    compile(&upstream, &units.build, &sources);
}

/// What bindgen binds of a unit or a header.
struct Binding<'a> {
    path: &'a str,
    functions: &'a [String],
    variables: &'a [String],
    /// Whether its `static` functions are wrapped: a header's are (the wrappers include the
    /// file, so a unit's cannot be).
    wrapped: bool,
}

impl Binding<'_> {
    /// The bindings' name: the file's, its dot an underscore (`random_c`, `math68k_h`).
    fn name(&self) -> String {
        Path::new(self.path)
            .file_name()
            .and_then(|name| name.to_str())
            .expect("a unit's file name")
            .replace('.', "_")
    }
}

/// The defines as `-D` arguments.
fn define_args(build: &Build) -> impl Iterator<Item = String> + '_ {
    build.defines.iter().map(|define| format!("-D{define}"))
}

/// Compiles `sources` into the static library.
fn compile(upstream: &Path, build: &Build, sources: &[PathBuf]) {
    let mut compiler = cc::Build::new();
    compiler
        .files(sources)
        .includes(build.include_dirs.iter().map(|dir| upstream.join(dir)))
        .opt_level(build.optimization)
        .warnings(false)
        // Upstream's C warns a great deal by design (its Makefile turns most of it off).
        .flag("-w")
        .flag("-include")
        .flag(upstream.join(&build.include).display().to_string());
    for flag in &build.flags {
        compiler.flag(flag);
    }
    for define in define_args(build) {
        compiler.flag(define);
    }
    compiler.compile(LIBRARY);
}

/// Binds `binding`'s functions and globals into `<out>/<name>.rs`, every function with the
/// `C-unwind` ABI (a stub may unwind through it). A header's `static` functions are wrapped
/// in a C file of their own, returned.
fn bind(upstream: &Path, build: &Build, binding: &Binding<'_>, out: &Path) -> Option<PathBuf> {
    let source = upstream.join(binding.path);
    let name = binding.name();
    let wrappers = out.join(format!("{name}{WRAPPERS_SUFFIX}"));
    let mut builder = bindgen::Builder::default()
        .header(source.display().to_string())
        .clang_arg("-include")
        .clang_arg(upstream.join(&build.include).display().to_string())
        .clang_args(
            build
                .include_dirs
                .iter()
                .map(|dir| format!("-I{}", upstream.join(dir).display())),
        )
        .clang_args(define_args(build))
        .clang_args(build.flags.iter().filter(|flag| flag.starts_with("-std=")))
        .override_abi(bindgen::Abi::CUnwind, ".*")
        .rust_edition(bindgen::RustEdition::Edition2024)
        .layout_tests(false)
        .wrap_static_fns(binding.wrapped)
        .wrap_static_fns_path(&wrappers)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));
    for function in binding.functions {
        builder = builder.allowlist_function(format!("^{function}$"));
    }
    for variable in binding.variables {
        builder = builder.allowlist_var(format!("^{variable}$"));
    }
    builder
        .generate()
        .unwrap_or_else(|error| panic!("bindgen on {}: {error}", source.display()))
        .write_to_file(out.join(format!("{name}.rs")))
        .expect("the bindings written");
    Some(wrappers.with_extension("c")).filter(|wrappers| binding.wrapped && wrappers.is_file())
}
