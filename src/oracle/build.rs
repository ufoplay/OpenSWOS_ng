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

/// The folder of the SDL shim, in `OUT_DIR`.
const SDL_SHIM_DIR: &str = "sdl_shim";

/// The SDL shim: upstream's SDL build (`SWOS_SDL_BUILD`) has headers include `<SDL.h>` for the
/// types they name (include/mod_api.h). The oracle compiles none of upstream's SDL code, so
/// those types are declared here, opaque or as SDL's integers: no SDL install is needed.
const SDL_SHIM: &str =
    "/* The oracle's SDL shim (src/oracle/build.rs): the types upstream's headers name. */
#ifndef OPENSWOS_ORACLE_SDL_SHIM
#define OPENSWOS_ORACLE_SDL_SHIM
#include <stdint.h>
typedef struct SDL_Renderer SDL_Renderer;
typedef struct SDL_Window SDL_Window;
typedef struct SDL_Texture SDL_Texture;
typedef int32_t SDL_Keycode;
typedef uint8_t Uint8;
typedef uint16_t Uint16;
typedef uint32_t Uint32;
typedef uint64_t Uint64;
typedef int32_t Sint32;
typedef int64_t Sint64;
#endif
";

/// The shim's file name.
const SDL_SHIM_FILE: &str = "SDL.h";

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
    /// Its globals, by name: bound as `static mut`.
    variables: Vec<String>,
    /// The constants it sees (enumerators, macros), by pattern.
    #[serde(default)]
    constants: Vec<String>,
    /// The types its tests read that no bound function or global reaches (a record behind a
    /// `void *`), by name.
    #[serde(default)]
    types: Vec<String>,
}

/// A header whose `static inline` functions the tests call: bound, not compiled (its
/// wrappers are).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    path: String,
    functions: Vec<String>,
}

/// A unit of upstream's data: linked as the units need it, and its globals listed here bound
/// as `static mut` (the tables `world_extract` reads).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DataUnit {
    path: String,
    #[serde(default)]
    variables: Vec<String>,
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
    let shim = sdl_shim(&out);
    for unit in &units.unit {
        let binding = Binding {
            path: &unit.path,
            functions: &unit.functions,
            variables: &unit.variables,
            constants: &unit.constants,
            types: &unit.types,
            wrapped: false,
        };
        bind(&upstream, &units.build, &binding, &out, &shim);
    }
    for data in units.data.iter().filter(|data| !data.variables.is_empty()) {
        let binding = Binding {
            path: &data.path,
            functions: &[],
            variables: &data.variables,
            constants: &[],
            types: &[],
            wrapped: false,
        };
        bind(&upstream, &units.build, &binding, &out, &shim);
    }
    for header in &units.header {
        let binding = Binding {
            path: &header.path,
            functions: &header.functions,
            variables: &[],
            constants: &[],
            types: &[],
            wrapped: true,
        };
        let wrappers = bind(&upstream, &units.build, &binding, &out, &shim)
            .unwrap_or_else(|| panic!("no static function of {} wrapped", header.path));
        sources.push(wrappers);
    }
    compile(&upstream, &units.build, &sources, &shim);
}

/// Writes the SDL shim into `out`, and returns its folder.
fn sdl_shim(out: &Path) -> PathBuf {
    let dir = out.join(SDL_SHIM_DIR);
    fs::create_dir_all(&dir).expect("the shim's folder");
    fs::write(dir.join(SDL_SHIM_FILE), SDL_SHIM).expect("the shim written");
    dir
}

/// What bindgen binds of a unit or a header.
struct Binding<'a> {
    path: &'a str,
    functions: &'a [String],
    variables: &'a [String],
    constants: &'a [String],
    types: &'a [String],
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

/// Compiles `sources` into the static library, the SDL shim in `shim`.
fn compile(upstream: &Path, build: &Build, sources: &[PathBuf], shim: &Path) {
    let mut compiler = cc::Build::new();
    compiler
        .files(sources)
        .includes(build.include_dirs.iter().map(|dir| upstream.join(dir)))
        .include(shim)
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

/// Binds `binding`'s functions, globals and types into `<out>/<name>.rs`, every function with the
/// `C-unwind` ABI (a stub may unwind through it). A header's `static` functions are wrapped
/// in a C file of their own, returned. The SDL shim is in `shim`.
fn bind(
    upstream: &Path,
    build: &Build,
    binding: &Binding<'_>,
    out: &Path,
    shim: &Path,
) -> Option<PathBuf> {
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
        .clang_arg(format!("-I{}", shim.display()))
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
    for variable in binding.variables.iter().chain(binding.constants) {
        builder = builder.allowlist_var(format!("^{variable}$"));
    }
    for type_name in binding.types {
        builder = builder.allowlist_type(format!("^{type_name}$"));
    }
    let code = builder
        .generate()
        .unwrap_or_else(|error| panic!("bindgen on {}: {error}", source.display()))
        .to_string();
    let code = as_statics(&code, binding.variables)
        .unwrap_or_else(|variable| panic!("{variable} is not a global of {}", source.display()));
    fs::write(out.join(format!("{name}.rs")), code).expect("the bindings written");
    Some(wrappers.with_extension("c")).filter(|wrappers| binding.wrapped && wrappers.is_file())
}

/// `code` with each of `variables` declared a `static mut`. bindgen declares a global the
/// unit defines with a value it can evaluate as a Rust `const` holding that value; it is
/// redeclared as the global it is, bindgen's type kept. Each variable must be declared one
/// way or the other: its name is returned otherwise.
fn as_statics(code: &str, variables: &[String]) -> Result<String, String> {
    let mut code = code.to_owned();
    for variable in variables {
        let constant = format!("pub const {variable}: ");
        let declared = format!("pub static mut {variable}: ");
        if let Some(start) = code.find(&constant) {
            let end = start + code[start..].find(';').ok_or_else(|| variable.clone())?;
            let declaration = &code[start + constant.len()..end];
            let (ty, _value) = declaration
                .split_once(" = ")
                .ok_or_else(|| variable.clone())?;
            let global = format!("unsafe extern \"C\" {{\n    {declared}{ty};\n}}");
            code.replace_range(start..=end, &global);
        } else if !code.contains(&declared) {
            return Err(variable.clone());
        }
    }
    Ok(code)
}
