// SPDX-License-Identifier: AGPL-3.0-only
//! API の契約を標準出力に書き出す。`docs/openapi.json` との差分を CI が見る。
#![allow(clippy::print_stdout)] // ここは標準出力が出口

use ashiato_server::collector_scope::{collector_allows, credential_exempt};
use axum::http::Method;
use utoipa::{
    openapi::{path::Operation, RefOr, Response},
    OpenApi as _,
};

fn add_forbidden_response(operation: &mut Operation) {
    operation
        .responses
        .responses
        .entry("403".to_owned())
        .or_insert_with(|| RefOr::T(Response::new("API の合言葉では通らない")));
}

/// 許可の一覧の外の口に 403 を足す。判定はサーバと同じ `collector_allows` を直に使う（写しを持たない。ST44 final review R14）。
fn add_collector_scope_responses(document: &mut utoipa::openapi::OpenApi) {
    for (path, item) in &mut document.paths.paths {
        let operations = [
            (Method::GET, item.get.as_mut()),
            (Method::HEAD, item.head.as_mut()),
            (Method::POST, item.post.as_mut()),
            (Method::PUT, item.put.as_mut()),
            (Method::PATCH, item.patch.as_mut()),
            (Method::DELETE, item.delete.as_mut()),
            (Method::OPTIONS, item.options.as_mut()),
            (Method::TRACE, item.trace.as_mut()),
        ];
        for (method, operation) in operations {
            let Some(operation) = operation else { continue };
            // 資格情報を求めない口は API の合言葉でも 403 にならないので足さない（ST44 D9）
            if credential_exempt(&method, path) || collector_allows(&method, path) {
                continue;
            }
            add_forbidden_response(operation);
        }
    }
}

fn main() -> anyhow::Result<()> {
    // 組み立ての印を bin に残す（`tools/check-build-stamp.sh server` が `strings` で読む）。使われない定数は消える
    std::hint::black_box(ashiato_server::BUILD_STAMP);
    let mut document = ashiato_server::ApiDoc::openapi();
    add_collector_scope_responses(&mut document);
    println!("{}", document.to_pretty_json()?);
    Ok(())
}
