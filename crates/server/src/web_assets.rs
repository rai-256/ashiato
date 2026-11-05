// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
//! `build.rs` が組み立てた画面の表を HTTP で配る（ST52 / design D13）。

use std::sync::Arc;

use axum::http::{header, HeaderValue, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::Extension;

#[derive(Clone, Copy, Debug)]
pub struct Asset {
    pub path: &'static str,
    pub body: &'static [u8],
    pub content_type: &'static str,
}

#[derive(Clone, Debug, Default)]
pub struct Assets(Arc<[Asset]>);

impl Assets {
    pub fn from_static(entries: &[(&'static str, &'static [u8], &'static str)]) -> Self {
        Self(
            entries
                .iter()
                .map(|&(path, body, content_type)| Asset {
                    path,
                    body,
                    content_type,
                })
                .collect(),
        )
    }

    pub fn empty() -> Self {
        Self::default()
    }

    fn find(&self, path: &str) -> Option<&Asset> {
        self.0.iter().find(|asset| asset.path == path)
    }
}

pub fn generated() -> Assets {
    let entries: &'static [Asset] = include!(concat!(env!("OUT_DIR"), "/web_assets.rs"));
    Assets(Arc::from(entries))
}

/// 画面のファイルを返す。無い道は 404。ある道でも GET / HEAD 以外は 405（route の既定と同じ。final review R14）。
pub async fn fallback(Extension(assets): Extension<Assets>, method: Method, uri: Uri) -> Response {
    let path = if uri.path() == "/" {
        "/index.html"
    } else {
        uri.path()
    };
    let Some(asset) = assets.find(path) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if method != Method::GET && method != Method::HEAD {
        let mut response = StatusCode::METHOD_NOT_ALLOWED.into_response();
        response
            .headers_mut()
            .insert(header::ALLOW, HeaderValue::from_static("GET,HEAD"));
        return response;
    }
    let mut response = Response::new(axum::body::Body::from(asset.body));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(asset.content_type),
    );
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'self'; frame-ancestors 'none'"),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}
