// SPDX-License-Identifier: AGPL-3.0-only
//! PC が止まっていた期間（FR-82 / 深掘り Q1 / design D10）。
//!
//! **扉 #14 が求める「データが無い日の意味」は、PC では電源とともに消える。**
//! 携帯端末では生存信号（FR-78）と受け手側の検知（FR-80）の 2 つで担われるが、
//! PC ではその両方が落ちるので、**C-02 自身が起動時に「前回の停止からの空白」を
//! 書き残さなければ、後から作る材料が無い**（Windows のイベントログは巡回して消える）。
//!
//! 印は動作中も更新する。**電源断・ブルースクリーンでは印を書く機会が無い**ので、
//! 最後に更新した時刻までしか判らない —— ずれは更新間隔（1 分）以内に収まる。
use anyhow::Context as _;
use chrono::{DateTime, Utc};

use crate::contract::{rfc3339, RecordKind, WindowPayload};

/// 印を更新する間隔（design D10 / Risks）。**区間の始まりのずれはこの幅に収まる。**
pub const TOUCH_INTERVAL_SEC: i64 = 60;

/// 「ここまでは動いていた」の印。
#[derive(Debug, Clone)]
pub struct Marker {
    path: std::path::PathBuf,
}

impl Marker {
    /// 置き場を決める。**未送信と同じディレクトリ**（消すときに取り残さない）。
    pub fn new(state_dir: &std::path::Path) -> Self {
        Self {
            path: state_dir.join("last-seen.txt"),
        }
    }

    /// 印を読む。**無ければ `None`**（初回起動）。
    ///
    /// **壊れていても空として扱わない** —— 空に倒すと、書きかけで落ちた 1 回が
    /// 「初回起動」に化けて、その停止期間が黙って消える。
    pub fn read(&self) -> anyhow::Result<Option<DateTime<Utc>>> {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => {
                let t = DateTime::parse_from_rfc3339(text.trim())
                    .with_context(|| format!("印が読めない: {}", self.path.display()))?;
                Ok(Some(t.with_timezone(&Utc)))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e).context("印を読めない"),
        }
    }

    /// 印を更新する（起動中は `TOUCH_INTERVAL_SEC` ごと）。
    pub fn touch(&self, now: DateTime<Utc>) -> anyhow::Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).context("置き場を作れない")?;
        }
        std::fs::write(&self.path, rfc3339(now)).context("印を書けない")
    }
}

/// 起動時の「PC が止まっていた」の記録（FR-82）。
///
/// - 印が無ければ**作らない**（初回起動。spec の Scenario）
/// - 印が未来を指していれば**作らない** —— 時計が戻ったときに
///   負の区間を残すと、読む側がどう扱うか決まっていない（扉 #5 の型）
pub fn powered_off_span(
    last_seen: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> Option<WindowPayload> {
    let last = last_seen?;
    if last >= now {
        return None;
    }
    let mut p = WindowPayload::new(RecordKind::PoweredOff, last);
    p.range_end = Some(rfc3339(now));
    Some(p)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn tmp_dir() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("ashiato-marker-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn t(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    /// 印は書けて読めて、**更新できる**（動作中は 1 分ごと。design D10）。
    #[test]
    fn last_seen_marker() {
        let dir = tmp_dir();
        let m = Marker::new(&dir);
        assert_eq!(m.read().unwrap(), None, "初回起動で印がある");
        m.touch(t("2026-09-12T23:00:00Z")).unwrap();
        assert_eq!(m.read().unwrap(), Some(t("2026-09-12T23:00:00Z")));
        m.touch(t("2026-09-12T23:01:00Z")).unwrap();
        assert_eq!(m.read().unwrap(), Some(t("2026-09-12T23:01:00Z")));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// 壊れた印は**エラー**にする（「初回起動」に化けさせない）。
    #[test]
    fn broken_marker_is_an_error() {
        let dir = tmp_dir();
        let m = Marker::new(&dir);
        std::fs::write(dir.join("last-seen.txt"), "きのう").unwrap();
        assert!(m.read().is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    /// **停止から起動までが 1 件になる**（FR-82 / 深掘り Q1）。
    ///
    /// Scenario: 起動時に止まっていた期間が 1 件残る
    /// Scenario: 初回起動では生成しない
    #[test]
    fn powered_off_span_is_one_record() {
        let off = powered_off_span(Some(t("2026-09-12T14:00:00Z")), t("2026-09-13T09:00:00Z"))
            .expect("止まっていた期間");
        assert_eq!(off.kind, RecordKind::PoweredOff);
        assert_eq!(off.at, "2026-09-12T14:00:00.000Z");
        assert_eq!(off.range_end.as_deref(), Some("2026-09-13T09:00:00.000Z"));
        assert!(
            off.app_name.is_none() && off.title.is_none(),
            "本文を持たない記録に本文が載っている"
        );

        // 初回起動（印が無い）
        assert!(powered_off_span(None, t("2026-09-13T09:00:00Z")).is_none());
        // 時計が戻った（印が未来）—— 負の区間を残さない
        assert!(
            powered_off_span(Some(t("2026-09-14T00:00:00Z")), t("2026-09-13T09:00:00Z")).is_none()
        );
    }
}
