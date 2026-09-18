// SPDX-License-Identifier: AGPL-3.0-only
//! マイアクティビティ製品名から凍結される論理ソース名を作る。
use sha2::{Digest, Sha256};
pub fn source_name(product: &str) -> String {
    let folded: String = product
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let normalized = folded.trim_matches('-');
    let suffix = if !normalized.is_empty() && product.is_ascii() {
        normalized.to_owned()
    } else {
        format!("u{:x}", Sha256::digest(product.as_bytes()))[..13].to_owned()
    };
    format!("c03-myactivity-{suffix}")
}
