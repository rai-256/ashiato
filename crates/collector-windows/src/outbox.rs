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
    broken: usize,
}

impl<T: Outboxable> Outbox<T> {
    /// 置き場を開き、**前回の続きを読み戻す**（起動と停止をまたいで保持する）。
    ///
    /// **無いときだけ空から始める。** 読めない（共有違反・権限・不正なバイト）ときは
    /// エラーにする —— 空として進むと、次の `remove` の書き直しが**溜まっていた全件を
    /// 上書きで消す**（review/code.md R20）。
    pub fn open(path: std::path::PathBuf) -> anyhow::Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).context("置き場を作れない")?;
        }
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(e).context("未送信の置き場を読めない"),
        };
        let mut pending = Vec::new();
        let mut broken = Vec::new();
        for l in text.lines().filter(|l| !l.trim().is_empty()) {
            match serde_json::from_str::<T>(l) {
                Ok(item) => pending.push(item),
                // **壊れた行は退避する。** 読めない 1 行のために残りを捨てない
                Err(_) => broken.push(l.to_string()),
            }
        }
        if !broken.is_empty() {
            // **退避先へは追記する**（上書きすると前回退避した行が消える。R30）
            use std::io::Write as _;
            let quarantine = path.with_extension("broken.jsonl");
            let mut f = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&quarantine)
                .context("退避先を開けない")?;
            for l in &broken {
                writeln!(f, "{l}").context("壊れた行を退避できない")?;
            }
            f.sync_all().context("退避先を同期できない")?;
        }
        let out = Self {
            path,
            pending,
            broken: broken.len(),
        };
        if out.broken > 0 {
            out.rewrite()?;
        }
        Ok(out)
    }

    /// 開いたときに退避した壊れた行の数（**ログに出す**。黙って退避しない）。
    pub fn broken_lines(&self) -> usize {
        self.broken
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
        writeln!(f, "{line}").context("置き場へ書けない")?;
        // **同期する**（電源断で最後の追記が消えないように。R31）
        f.sync_data().context("置き場を同期できない")
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
        // 一時ファイルに書いて同期してから置き換える —— 途中で落ちても
        // 「書きかけの全件」にならない
        crate::fsutil::atomic_write(&self.path, text.as_bytes())
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
        let o: Outbox<IngestRequest> = Outbox::open(path.clone()).unwrap();
        assert_eq!(o.snapshot(), std::slice::from_ref(&good));
        assert_eq!(o.broken_lines(), 1, "退避した件数が数えられていない");
        // 2 回目の退避が 1 回目を消さない（追記）
        std::fs::write(
            &path,
            format!("{{二つ目\n{}\n", serde_json::to_string(&good).unwrap()),
        )
        .unwrap();
        let _o: Outbox<IngestRequest> = Outbox::open(path).unwrap();
        let broken = std::fs::read_to_string(dir.join("outbox.broken.jsonl")).unwrap();
        assert!(
            broken.contains("書きかけ") && broken.contains("二つ目"),
            "前回退避した行が消えた: {broken}"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    /// **読めない置き場を空として開かない**（R20。空で開くと次の書き直しで全件消える）。
    #[test]
    fn unreadable_outbox_is_an_error_not_empty() {
        let dir = tmp_dir();
        let path = dir.join("outbox.jsonl");
        // 不正な UTF-8（読み取りが失敗する）
        std::fs::write(&path, [0xff, 0xfe, 0xfd, b'\n']).unwrap();
        assert!(Outbox::<IngestRequest>::open(path.clone()).is_err());
        assert_eq!(
            std::fs::read(&path).unwrap(),
            [0xff, 0xfe, 0xfd, b'\n'],
            "中身が書き換わった"
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
