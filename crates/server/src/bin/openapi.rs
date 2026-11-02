// SPDX-License-Identifier: AGPL-3.0-only
//! API の契約を標準出力に書き出す。`docs/openapi.json` との差分を CI が見る。
#![allow(clippy::print_stdout)] // ここは標準出力が出口

use utoipa::{
    openapi::{path::Operation, RefOr, Response},
    OpenApi as _,
};

fn collector_allowed(method: &str, path: &str) -> bool {
    ashiato_server::collector_scope::COLLECTOR_ALLOWED
        .iter()
        .any(|(allowed_method, allowed_path)| {
            allowed_method.as_str().eq_ignore_ascii_case(method) && *allowed_path == path
        })
}

fn add_forbidden_response(operation: &mut Operation) {
    operation
        .responses
        .responses
        .entry("403".to_owned())
        .or_insert_with(|| RefOr::T(Response::new("API の合言葉では通らない")));
}

fn add_collector_scope_responses(document: &mut utoipa::openapi::OpenApi) {
    for (path, item) in &mut document.paths.paths {
        if path == "/healthz" {
            // /healthz は API の合言葉でも通るため、一覧外の拒否には当たらない。
            continue;
        }
        if !collector_allowed("get", path) {
            if let Some(operation) = item.get.as_mut() {
                add_forbidden_response(operation);
            }
        }
        if !collector_allowed("head", path) {
            if let Some(operation) = item.head.as_mut() {
                add_forbidden_response(operation);
            }
        }
        if !collector_allowed("post", path) && path != "/session" {
            if let Some(operation) = item.post.as_mut() {
                add_forbidden_response(operation);
            }
        }
        if !collector_allowed("put", path) {
            if let Some(operation) = item.put.as_mut() {
                add_forbidden_response(operation);
            }
        }
        if let Some(operation) = item.patch.as_mut() {
            add_forbidden_response(operation);
        }
        if path != "/session" {
            if let Some(operation) = item.delete.as_mut() {
                add_forbidden_response(operation);
            }
        }
        if let Some(operation) = item.options.as_mut() {
            add_forbidden_response(operation);
        }
        if let Some(operation) = item.trace.as_mut() {
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
