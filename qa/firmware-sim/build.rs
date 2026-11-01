//! Rebuilds the firmware crate root for the host from the real `main.rs`.
//!
//! The module tree, feature gates and `firmware_main` come straight from the
//! firmware. Crate-level attributes are dropped because they are only legal in
//! a real crate root, and `fault` is dropped because the host panic runtime
//! owns the panic handler. An included file cannot name a `macro_export` macro
//! by path, so `log!` is re-exported at crate level instead.
use std::{env, fs, path::Path};

const FIRMWARE: &str = "../../apps/kaskold-hardware";
const RELEASE_POLICY_KEYS: [&str; 2] = ["KASKOLD_SECURITY_VERSION", "KASKOLD_ESPTOOL_VERSION"];

fn main() {
    let manifest = env::var("CARGO_MANIFEST_DIR").expect("cargo sets the manifest directory");
    let src = Path::new(&manifest).join(FIRMWARE).join("src");
    println!("cargo:rerun-if-changed={}", src.join("main.rs").display());
    println!("cargo:rerun-if-changed={FIRMWARE}/release-policy.env");
    println!("cargo:rustc-env=KASKOLD_BUILD_COMMIT=0000000");
    println!("cargo:rustc-env=KASKOLD_WORKFLOW_E2E_FROM=1");
    emit_release_policy();
    let root = fs::read_to_string(src.join("main.rs")).expect("firmware main.rs");
    let mut rewritten: Vec<String> = root
        .lines()
        .filter_map(|line| rewrite(line, &src))
        .collect();
    rewritten.push("pub(crate) use log;".to_owned());
    let out = Path::new(&env::var("OUT_DIR").expect("cargo sets OUT_DIR")).join("firmware_root.rs");
    fs::write(out, rewritten.join("\n")).expect("generated crate root");
}

fn emit_release_policy() {
    let path = format!("{FIRMWARE}/release-policy.env");
    let policy = fs::read_to_string(path).expect("firmware release policy");
    for (key, value) in policy
        .lines()
        .filter_map(|line| line.trim().split_once('='))
    {
        if RELEASE_POLICY_KEYS.contains(&key) {
            println!("cargo:rustc-env={key}={value}");
        }
    }
}

fn rewrite(line: &str, src: &Path) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.starts_with("#![") || trimmed == "mod fault;" || trimmed == "#[macro_export]" {
        return None;
    }
    let Some(name) = trimmed
        .strip_prefix("mod ")
        .and_then(|rest| rest.strip_suffix(';'))
    else {
        return Some(line.to_owned());
    };
    let single = src.join(format!("{name}.rs"));
    let module = if single.exists() {
        single
    } else {
        src.join(name).join("mod.rs")
    };
    Some(format!(
        "#[path = {:?}]\nmod {name};",
        module.display().to_string()
    ))
}
