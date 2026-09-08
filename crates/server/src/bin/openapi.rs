// SPDX-License-Identifier: AGPL-3.0-only
//! API の契約を標準出力に書き出す。`docs/openapi.json` との差分を CI が見る。
#![allow(clippy::print_stdout)] // ここは標準出力が出口

use utoipa::OpenApi as _;

fn main() -> anyhow::Result<()> {
    println!("{}", ashiato_server::ApiDoc::openapi().to_pretty_json()?);
    Ok(())
}
