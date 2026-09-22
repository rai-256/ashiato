// SPDX-License-Identifier: AGPL-3.0-only
//! 送った履歴と消失判定を保つ帳面。
use std::collections::BTreeMap;

use anyhow::Context as _;
use chrono::{DateTime, Utc};

/// 訪問 1 件について、次の読取りと比べるために残す最小の情報。
///
/// URL と題名は本文から消せるため、帳面には内容ハッシュだけを残す。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LedgerVisit {
    pub content_hash: String,
    pub at: DateTime<Utc>,
    pub foreign: bool,
    pub excluded: bool,
}

/// プロファイル単位で永続化する比較状態。
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Ledger {
    pub visits: BTreeMap<String, LedgerVisit>,
    pub max_visit_id: Option<i64>,
    /// ディレクトリ名と、前回送った表示名の対応。表示名を訪問へ複写しない。
    pub profile_names: BTreeMap<String, Option<String>>,
}

impl Ledger {
    pub fn record_visit(
        &mut self,
        external_id: &str,
        content_hash: &str,
        at: DateTime<Utc>,
        foreign: bool,
        excluded: bool,
    ) {
        self.visits.insert(
            external_id.to_owned(),
            LedgerVisit {
                content_hash: content_hash.to_owned(),
                at,
                foreign,
                excluded,
            },
        );
    }

    pub fn set_profile_name(&mut self, directory: &str, name: Option<String>) {
        self.profile_names.insert(directory.to_owned(), name);
    }
}

/// 帳面の置き場。一時ファイルからの置換で、前の帳面か新しい帳面の一方を残す。
#[derive(Debug)]
pub struct LedgerStore {
    path: std::path::PathBuf,
    ledger: Ledger,
}

impl LedgerStore {
    pub fn open(path: std::path::PathBuf) -> anyhow::Result<Self> {
        let ledger = match std::fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice(&bytes) {
                Ok(ledger) => ledger,
                Err(_) => {
                    let quarantine = path.with_extension("broken.ledger");
                    std::fs::rename(&path, &quarantine).with_context(|| {
                        format!("壊れた履歴帳面を退避できない: {}", path.display())
                    })?;
                    Ledger::default()
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ledger::default(),
            Err(error) => return Err(error).context("履歴帳面を読めない"),
        };
        Ok(Self { path, ledger })
    }

    pub fn ledger(&self) -> &Ledger {
        &self.ledger
    }

    pub fn ledger_mut(&mut self) -> &mut Ledger {
        &mut self.ledger
    }

    pub fn save(&self) -> anyhow::Result<()> {
        crate::fsutil::atomic_write(&self.path, &serde_json::to_vec(&self.ledger)?)
            .context("履歴帳面を書けない")
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    /// 帳面には本文を入れない。本文を物理削除しても URL・題名が残る形を防ぐ。
    #[test]
    fn history_ledger_has_no_url_or_title() {
        let mut ledger = Ledger::default();
        ledger.record_visit(
            "v1:visit:abc",
            "content-hash",
            chrono::Utc::now(),
            false,
            false,
        );
        ledger.set_profile_name("Default", Some("個人".into()));
        let text = serde_json::to_string(&ledger).unwrap();
        assert!(!text.contains("url"));
        assert!(!text.contains("title"));
        assert!(!text.contains("https://"));
    }

    /// 壊れた帳面は比較を一度あきらめて退避し、新しい取得を止めない。
    #[test]
    fn history_ledger_broken_is_quarantined() {
        let dir = temp_dir();
        let path = dir.join("chrome/Default.ledger");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"{broken").unwrap();
        let ledger = LedgerStore::open(path.clone()).unwrap();
        assert!(ledger.ledger().visits.is_empty());
        assert!(!path.exists());
        assert_eq!(
            std::fs::read(path.with_extension("broken.ledger")).unwrap(),
            b"{broken"
        );
        std::fs::remove_dir_all(dir).ok();
    }

    fn temp_dir() -> std::path::PathBuf {
        let path =
            std::env::temp_dir().join(format!("ashiato-history-ledger-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).unwrap();
        path
    }
}
