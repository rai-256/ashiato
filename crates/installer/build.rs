use std::path::Path;

fn main() {
    println!("cargo:rerun-if-env-changed=ASHIATO_RELEASE");
    println!("cargo:rustc-check-cfg=cfg(ashiato_test_build)");
    if std::env::var("ASHIATO_RELEASE").as_deref() != Ok("1") {
        println!("cargo:rustc-cfg=ashiato_test_build");
    }
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let version = ashiato_build_stamp::version(env!("CARGO_PKG_VERSION"))
        .expect("CARGO_PKG_VERSION は作業場の版番号の形である必要があります");
    let stamp = ashiato_build_stamp::stamp(
        &version,
        &std::env::var("PROFILE").unwrap_or_default(),
        &repo,
        &[
            "crates/installer",
            "crates/build-stamp",
            "Cargo.toml",
            "Cargo.lock",
        ],
    );
    ashiato_build_stamp::emit_rerun(
        &repo,
        &[
            "crates/installer",
            "crates/build-stamp",
            "Cargo.toml",
            "Cargo.lock",
        ],
    );
    println!("cargo:rustc-env=ASHIATO_BUILD_STAMP={stamp}");
    println!("cargo:rustc-env=ASHIATO_VERSION={version}");
}
