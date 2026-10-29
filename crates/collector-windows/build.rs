// SPDX-License-Identifier: AGPL-3.0-only
//! 組み立ての印（ST49 / design D5）を `ASHIATO_BUILD_STAMP` として焼き込む。S-01 の `build.rs` と同じ作り方。
use std::{env, path::Path};

/// C-02 のソースの道。`.dirty` の判定と、cargo への作り直しの印の両方に使う。
const STAMP_SOURCES: &[&str] = &[
    "crates/collector-windows",
    "crates/build-stamp",
    "Cargo.toml",
    "Cargo.lock",
];

fn main() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let stamp = ashiato_build_stamp::stamp(
        env!("CARGO_PKG_VERSION"),
        &env::var("PROFILE").unwrap_or_default(),
        &repo,
        STAMP_SOURCES,
    );
    ashiato_build_stamp::emit_rerun(&repo, STAMP_SOURCES);
    println!("cargo:rustc-env=ASHIATO_BUILD_STAMP={stamp}");
}
