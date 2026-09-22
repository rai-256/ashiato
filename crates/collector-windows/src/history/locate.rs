// SPDX-License-Identifier: AGPL-3.0-only
//! 既知のブラウザ置き場を見つける。
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Browser {
    Chrome,
    Edge,
    Brave,
    Vivaldi,
    Opera,
    Firefox,
}

impl Browser {
    pub const ALL: [Self; 6] = [
        Self::Chrome,
        Self::Edge,
        Self::Brave,
        Self::Vivaldi,
        Self::Opera,
        Self::Firefox,
    ];
    pub fn base(self, local: &Path, roaming: &Path) -> PathBuf {
        match self {
            Self::Chrome => local.join("Google/Chrome/User Data"),
            Self::Edge => local.join("Microsoft/Edge/User Data"),
            Self::Brave => local.join("BraveSoftware/Brave-Browser/User Data"),
            Self::Vivaldi => local.join("Vivaldi/User Data"),
            Self::Opera => roaming.join("Opera Software/Opera Stable"),
            Self::Firefox => roaming.join("Mozilla/Firefox/Profiles"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub browser: Browser,
    pub directory: String,
    pub path: PathBuf,
}

/// ディレクトリ名から表示名への対応。表示名は訪問ごとには載せない。
pub fn profile_names(browser: Browser, base: &Path) -> std::collections::BTreeMap<String, String> {
    let mut out = std::collections::BTreeMap::new();
    let text = match browser {
        Browser::Firefox => std::fs::read_to_string(base.join("profiles.ini")),
        _ => std::fs::read_to_string(base.join("Local State")),
    };
    let Ok(text) = text else {
        return out;
    };
    if browser == Browser::Firefox {
        let mut name = None;
        let mut path = None;
        for line in text.lines().chain(std::iter::once("")) {
            if let Some(v) = line.strip_prefix("Name=") {
                name = Some(v.to_owned())
            } else if let Some(v) = line.strip_prefix("Path=") {
                path = Some(v.rsplit(['/', '\\']).next().unwrap_or(v).to_owned())
            } else if line.is_empty() {
                if let (Some(p), Some(n)) = (path.take(), name.take()) {
                    out.insert(p, n);
                }
            }
        }
    } else if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
        if let Some(map) = value
            .pointer("/profile/info_cache")
            .and_then(serde_json::Value::as_object)
        {
            for (dir, v) in map {
                if let Some(name) = v.get("name").and_then(serde_json::Value::as_str) {
                    out.insert(dir.clone(), name.to_owned());
                }
            }
        }
    }
    out
}

/// 既知の 6 ブラウザを発見する。Chromium は直下 1 段、Firefox は走査と ini の和を取る。
pub fn locate(local: &Path, roaming: &Path) -> Vec<Profile> {
    let mut found = Vec::new();
    for browser in Browser::ALL {
        let base = browser.base(local, roaming);
        if browser == Browser::Firefox {
            scan(browser, &base, "places.sqlite", &mut found);
            scan_ini(
                browser,
                &roaming.join("Mozilla/Firefox/profiles.ini"),
                &mut found,
            );
        } else {
            scan(browser, &base, "History", &mut found);
            if browser == Browser::Opera {
                scan(browser, &base.join("_side_profiles"), "History", &mut found);
            }
        }
    }
    found.sort_by(|a, b| a.path.cmp(&b.path));
    found.dedup_by(|a, b| a.path == b.path);
    found
}

fn scan(browser: Browser, base: &Path, file: &str, out: &mut Vec<Profile>) {
    let Ok(entries) = std::fs::read_dir(base) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        let db = p.join(file);
        if p.is_dir() && db.is_file() {
            out.push(Profile {
                browser,
                directory: entry.file_name().to_string_lossy().into_owned(),
                path: db,
            });
        }
    }
    if browser == Browser::Opera && base.join(file).is_file() {
        out.push(Profile {
            browser,
            directory: "Default".into(),
            path: base.join(file),
        });
    }
}

fn scan_ini(browser: Browser, ini: &Path, out: &mut Vec<Profile>) {
    let Ok(text) = std::fs::read_to_string(ini) else {
        return;
    };
    let parent = ini.parent().unwrap_or_else(|| Path::new(""));
    let relative = !text.lines().any(|line| line.trim() == "IsRelative=0");
    for line in text.lines().filter_map(|line| line.strip_prefix("Path=")) {
        let dir = if relative {
            parent.join(line)
        } else {
            PathBuf::from(line)
        };
        let db = dir.join("places.sqlite");
        if db.is_file() {
            out.push(Profile {
                browser,
                directory: dir
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                path: db,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn history_locate_all_known_browsers() {
        // Scenario: 6 つのブラウザの既知の置き場にあるプロファイルが全部見つかる
        let root = temp();
        for browser in Browser::ALL {
            fixture(&root, browser, "Default");
        }
        let found = locate(&root.join("local"), &root.join("roaming"));
        assert_eq!(found.len(), 6);
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn history_locate_firefox_ini_path() {
        // Scenario: Firefox の一覧にある既定の外の置き場も見つかる
        let root = temp();
        let local = root.join("local");
        let roaming = root.join("roaming");
        let custom = root.join("outside");
        std::fs::create_dir_all(&custom).unwrap();
        std::fs::write(custom.join("places.sqlite"), []).unwrap();
        std::fs::create_dir_all(roaming.join("Mozilla/Firefox")).unwrap();
        std::fs::write(
            roaming.join("Mozilla/Firefox/profiles.ini"),
            format!("[Profile0]\nPath={}\nIsRelative=0\n", custom.display()),
        )
        .unwrap();
        assert!(locate(&local, &roaming)
            .iter()
            .any(|p| p.path == custom.join("places.sqlite")));
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn history_locate_multiple_profiles() {
        // Scenario: 複数のブラウザと複数のプロファイルの履歴が全部入る
        let root = temp();
        fixture(&root, Browser::Chrome, "Default");
        fixture(&root, Browser::Chrome, "Profile 1");
        fixture(&root, Browser::Firefox, "abc");
        assert_eq!(locate(&root.join("local"), &root.join("roaming")).len(), 3);
        std::fs::remove_dir_all(root).ok();
    }
    #[test]
    fn history_profiles_map() {
        // Scenario: プロファイルの表示名との対応が残る
        // Scenario: プロファイルの表示名を変えると新しい対応が残る
        let root = temp();
        let base = root.join("local/Google/Chrome/User Data");
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(
            base.join("Local State"),
            r#"{"profile":{"info_cache":{"Default":{"name":"個人"}}}}"#,
        )
        .unwrap();
        assert_eq!(
            profile_names(Browser::Chrome, &base).get("Default"),
            Some(&"個人".to_string())
        );
        std::fs::remove_dir_all(root).ok();
    }

    fn temp() -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("ashiato-history-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&p).unwrap();
        p
    }
    fn fixture(root: &std::path::Path, browser: Browser, profile: &str) {
        let base = browser.base(&root.join("local"), &root.join("roaming"));
        let path = if browser == Browser::Firefox {
            base.join(profile).join("places.sqlite")
        } else {
            base.join(profile).join("History")
        };
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, []).unwrap();
    }
}
