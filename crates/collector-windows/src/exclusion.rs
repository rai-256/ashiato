// SPDX-License-Identifier: AGPL-3.0-only
//! 収集から除外する対象（FR-83 / 深掘り Q5 / design D11）。
//!
//! **判定は送る前に行う。** 送ってからサーバ側で消すのでは守りにならない ——
//! 格納された記録は移行 0004 のトリガで凍結され、バックアップ（FR-66 / FR-68）は
//! 取り込み口の後段にある（第 2 回 Q9 で本人が「PC 側で落とす」を選んだ）。
//!
//! **既定は空。** 深掘り Q3 で既定の感度を緩い側（外部 AI 可）に置いたので、
//! ここが唯一の「そもそも記録しない」手段になっている。初期登録の手順は
//! `crates/collector-windows/README.md`。
use serde::{Deserialize, Serialize};

use crate::engine::Foreground;

/// 除外する対象の指し方。
///
/// **3 通りとも要る** —— 同じソフトが版で別のパスに入り（`ProcessName`）、
/// 同じプロセスの中に残したい窓と残したくない窓がある（`TitleContains`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "match", rename_all = "kebab-case")]
pub enum Rule {
    /// 実行ファイルのパスの完全一致。**大文字小文字を無視する**（Windows の規則）
    ExePath { value: String },
    /// プロセス名の完全一致。同上
    ProcessName { value: String },
    /// ウィンドウ題名の部分一致。**シークレットウィンドウのように
    /// 「同じソフトの中の一部の窓だけ」を落とすため**
    TitleContains { value: String },
}

impl Rule {
    fn hits(&self, fg: &Foreground) -> bool {
        match self {
            Self::ExePath { value } => fg.exe_path.eq_ignore_ascii_case(value),
            Self::ProcessName { value } => fg.process_name.eq_ignore_ascii_case(value),
            Self::TitleContains { value } => {
                !value.is_empty() && fg.title.to_lowercase().contains(&value.to_lowercase())
            }
        }
    }
}

/// 除外の登録。**既定は空**（tasks 6.1）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exclusions {
    /// 登録された規則。1 つでも当たれば除外
    #[serde(default)]
    pub rules: Vec<Rule>,
}

impl Exclusions {
    /// 置き場から読む。**ファイルが無ければ空**（既定は何も除外しない）。
    ///
    /// **壊れた内容は空に倒さずエラーにする。** 空に倒すと、書き間違いで
    /// 除外が黙って外れ、**残したくなかった題名と URL が外部 AI 可のまま残る**
    /// （深掘り Q3 の代償を広げる）。
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(text) => Ok(serde_json::from_str(&text)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.into()),
        }
    }

    /// この前景を除外するか。
    pub fn hits(&self, fg: &Foreground) -> bool {
        self.rules.iter().any(|r| r.hits(fg))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::engine::UrlRead;

    fn fg(exe: &str, proc_name: &str, title: &str) -> Foreground {
        Foreground {
            app_name: "アプリ".into(),
            exe_path: exe.into(),
            process_name: proc_name.into(),
            title: title.into(),
            url: UrlRead::NotBrowser,
        }
    }

    /// **既定は空**（tasks 6.1）。登録が無ければ何も除外しない。
    #[test]
    fn exclusion_empty_by_default() {
        let e = Exclusions::default();
        assert!(e.rules.is_empty());
        assert!(!e.hits(&fg(r"C:\x\a.exe", "a.exe", "何か")));

        // 置き場が無いときも空（初回起動）
        let dir = std::env::temp_dir().join(format!("ashiato-ex-{}", uuid::Uuid::new_v4()));
        assert_eq!(
            Exclusions::load(&dir.join("exclusions.json")).unwrap(),
            Exclusions::default()
        );
    }

    /// 3 通りの指し方が効く。**パスとプロセス名は大文字小文字を無視する。**
    #[test]
    fn rules_hit_by_path_name_and_title() {
        let e = Exclusions {
            rules: vec![
                Rule::ExePath {
                    value: r"C:\Program Files\pass\pass.exe".into(),
                },
                Rule::ProcessName {
                    value: "vault.exe".into(),
                },
                Rule::TitleContains {
                    value: "シークレット".into(),
                },
            ],
        };
        assert!(e.hits(&fg(r"c:\program files\PASS\Pass.EXE", "pass.exe", "金庫")));
        assert!(e.hits(&fg(r"C:\other\v.exe", "VAULT.exe", "金庫")));
        assert!(e.hits(&fg(r"C:\b\b.exe", "b.exe", "シークレット ウィンドウ")));
        assert!(!e.hits(&fg(r"C:\b\b.exe", "b.exe", "通常のウィンドウ")));
    }

    /// 壊れた登録は**空に倒さない**（除外が黙って外れるのを防ぐ）。
    #[test]
    fn broken_registration_is_an_error_not_empty() {
        let path = std::env::temp_dir().join(format!("ashiato-ex-{}.json", uuid::Uuid::new_v4()));
        std::fs::write(&path, "{ これは JSON ではない").unwrap();
        assert!(Exclusions::load(&path).is_err());
        std::fs::remove_file(&path).ok();
    }
}
