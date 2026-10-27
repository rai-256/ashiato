// SPDX-License-Identifier: AGPL-3.0-only
//! API の契約を標準出力に書き出す。`docs/openapi.json` との差分を CI が見る。
#![allow(clippy::print_stdout)] // ここは標準出力が出口

use utoipa::OpenApi as _;

fn main() -> anyhow::Result<()> {
    // 組み立ての印を bin に残す（`tools/check-build-stamp.sh server` が `strings` で読む）。使われない定数は消える
    std::hint::black_box(ashiato_server::BUILD_STAMP);
    println!("{}", ashiato_server::ApiDoc::openapi().to_pretty_json()?);
    Ok(())
}
