// SPDX-License-Identifier: AGPL-3.0-only
//! まだ送れていない記録の置き場（FR-10 / design D3）。
//!
//! **上限も破棄も持たない。** 上限を置く判断（`discarded`）は ST04 の担当で、
//! ST07 では扉を開けたままにする —— **捨てなければ何も失われない。**
//!
//! **S-01 と同じ PC で動いていても要る。** 網は切れないが、サーバの再起動・
//! 移行の適用・DB の停止は同じ PC でも起きる。ウィンドウは 1 日数千件出るので、
//! 数分の停止でも数百件になる。
//!
//! 形は C-01 の `FileOutboxStore` と同じ **追記のみの JSONL**。
//! **書きかけの行は読み飛ばして退避する** —— 1 行が壊れただけで
//! 残り全部を捨てるのは、捨てないという約束に反する。
use anyhow::Context as _;

use crate::contract::Outboxable;

/// 追記のみの JSONL に積む置き場。
#[derive(Debug)]
pub struct Outbox<T> {
    path: std::path::PathBuf,
    pending: Vec<T>,
}

impl<T: Outboxable> Outbox<T> {
    /// 置き場を開き、**前回の続きを読み戻す**（起動と停止をまたいで保持する）。
    pub fn open(path: std::path::PathBuf) -> anyhow::Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).context("置き場を作れない")?;
        }
        let mut pending = Vec::new();
        let mut broken = Vec::new();
        if let Ok(text) = std::fs::read_to_string(&path) {
            for l in text.lines().filter(|l| !l.trim().is_empty()) {
                match serde_json::from_str::<T>(l) {
                    Ok(item) => pending.push(item),
                    // **壊れた行は退避する。** 読めない 1 行のために残りを捨てない
                    Err(_) => broken.push(l.to_string()),
                }
            }
        }
        if !broken.is_empty() {
            let quarantine = path.with_extension("broken.jsonl");
            let mut text = std::fs::read_to_string(&quarantine).unwrap_or_default();
            for l in &broken {
                text.push_str(l);
                text.push('\n');
            }
            std::fs::write(&quarantine, text).context("壊れた行を退避できない")?;
        }
        let out = Self { path, pending };
        if !broken.is_empty() {
            out.rewrite()?;
        }
        Ok(out)
    }

    /// 積む。**件数でも内容でもふるい落とさない。**
    pub fn add(&mut self, item: T) -> anyhow::Result<()> {
        let line = serde_json::to_string(&item)?;
        self.pending.push(item);
        // **追記する** —— 全件書き直しは未送信が伸びたときに書き込みを焼く
        use std::io::Write as _;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .context("置き場を開けない")?;
        writeln!(f, "{line}").context("置き場へ書けない")
    }

    /// いま溜まっているもの。
    pub fn snapshot(&self) -> &[T] {
        &self.pending
    }

    /// 溜まっている件数。
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    /// 空か。
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// 送れたものだけを取り除く。**識別子で消す** ——
    /// 送信の途中で新しい記録が積まれても取り違えないため。
    pub fn remove(&mut self, ids: &[uuid::Uuid]) -> anyhow::Result<()> {
        if ids.is_empty() {
            return Ok(());
        }
        self.pending.retain(|i| !ids.contains(&i.id()));
        self.rewrite()
    }

    fn rewrite(&self) -> anyhow::Result<()> {
        let mut text = String::new();
        for i in &self.pending {
            text.push_str(&serde_json::to_string(i)?);
            text.push('\n');
        }
        // **一時ファイルに書いてから置き換える** —— 途中で落ちても
        // 「書きかけの全件」にならない
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, text).context("書き直せない")?;
        std::fs::rename(&tmp, &self.path).context("置き換えられない")
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::contract::{IngestRequest, WindowPayload};

    fn tmp_dir() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("ashiato-outbox-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn req(n: u32) -> IngestRequest {
        let at = chrono::DateTime::parse_from_rfc3339("2026-09-13T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc)
            + chrono::Duration::seconds(n as i64);
        let mut p = WindowPayload::new(crate::contract::RecordKind::Foreground, at);
        p.app_name = Some(format!("app-{n}"));
        IngestRequest::of(
            &p,
            uuid::Uuid::nil(),
            "dev-1",
            at,
            &crate::config::Zone {
                id: "Asia/Tokyo".into(),
                offset_min: 540,
            },
        )
        .unwrap()
    }

    /// **起動と停止をまたいで残る**（インスタンスの中だけに積まない。ST01 の実測）。
    #[test]
    fn outbox_survives_restart() {
        let dir = tmp_dir();
        let path = dir.join("outbox.jsonl");
        let a = req(1);
        let b = req(2);
        {
            let mut o: Outbox<IngestRequest> = Outbox::open(path.clone()).unwrap();
            o.add(a.clone()).unwrap();
            o.add(b.clone()).unwrap();
            assert_eq!(o.len(), 2);
        }
        let mut o: Outbox<IngestRequest> = Outbox::open(path.clone()).unwrap();
        assert_eq!(o.snapshot(), &[a.clone(), b.clone()]);
        o.remove(&[a.id()]).unwrap();
        let o: Outbox<IngestRequest> = Outbox::open(path).unwrap();
        assert_eq!(o.snapshot(), &[b], "取り除いた結果が置き場に残っていない");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// **壊れた 1 行のために残りを捨てない**（捨てないという約束を守る）。
    #[test]
    fn broken_line_is_quarantined_not_dropped() {
        let dir = tmp_dir();
        let path = dir.join("outbox.jsonl");
        let good = req(3);
        std::fs::write(
            &path,
            format!("{{書きかけ\n{}\n", serde_json::to_string(&good).unwrap()),
        )
        .unwrap();
        let o: Outbox<IngestRequest> = Outbox::open(path).unwrap();
        assert_eq!(o.snapshot(), &[good]);
        let broken = std::fs::read_to_string(dir.join("outbox.broken.jsonl")).unwrap();
        assert!(broken.contains("書きかけ"), "壊れた行が退避されていない");
        std::fs::remove_dir_all(&dir).ok();
    }
}
