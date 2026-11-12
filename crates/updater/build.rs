use std::{env, path::Path};

fn main() {
    println!("cargo:rustc-check-cfg=cfg(ashiato_test_build)");
    if env::var("ASHIATO_RELEASE").as_deref() != Ok("1") {
        println!("cargo:rustc-cfg=ashiato_test_build");
    }
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let inputs = [
        "crates/updater",
        "crates/update-check",
        "crates/build-stamp",
        "Cargo.toml",
        "Cargo.lock",
    ];
    let version = ashiato_build_stamp::version(env!("CARGO_PKG_VERSION")).expect("版番号が不正");
    let stamp = ashiato_build_stamp::stamp(
        &version,
        &env::var("PROFILE").unwrap_or_default(),
        &repo,
        &inputs,
    );
    ashiato_build_stamp::emit_rerun(&repo, &inputs);
    println!("cargo:rustc-env=ASHIATO_BUILD_STAMP={stamp}");
    println!("cargo:rustc-env=ASHIATO_VERSION={version}");
}
