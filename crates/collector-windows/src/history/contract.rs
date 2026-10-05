// SPDX-License-Identifier: AGPL-3.0-only
//! ブラウザ履歴を取り込み口へ送る形（ST08 design D4 / D5 / D6）。
//!
//! 1 件は `visit` / `vanished` / `excluded` / `profiles` のどれか。**`raw` は収集側が組んだ JSON を文字列のまま**
//! 送り、`payload` は同じ中身から作る（ST07 D1 と同じ）。直列化の形は `visit_payload_shape_is_pinned` が
//! 1 文字単位で固定する —— 形が変わると、同じ訪問が「内容が変わった」として版を積む。
//!
//! 識別子は design D6 の式そのもの（第 2 回 Q5: 組全体を 1 つのハッシュにする）。区切りは本文に現れない `\x1f`。
use std::collections::BTreeMap;

use anyhow::Context as _;
use chrono::{DateTime, SecondsFormat, Utc};
use serde::Serialize;
use sha2::{Digest as _, Sha256};

use crate::contract::IngestRequest;
use crate::history::fetch::VanishedVisit;
use crate::history::locate::Browser;
use crate::history::read::ReadVisit;

/// 登録簿（`core.source`）にある履歴の論理ソース名。
pub const LOGICAL_SOURCE: &str = "c02-browser-history";
/// `tz_basis`（design D5）。タイムゾーンは**その内容を読んだ取得のときの PC のもの**。
pub const TZ_BASIS: &str = "collected-at";

/// 取り込み口へ送る履歴の 1 件。識別子は本文と分けて持ち、受け口の更新キーにする（design D8）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Visit {
    pub external_id: String,
    pub payload: VisitPayload,
    /// `payload` を直列化した文字列。`raw` と内容のハッシュの元
    pub raw: String,
    /// 出来事の時刻（`payload.at` と同じ値）。帳面の訪問時刻に使う（文字列から読み直さない）
    pub at: DateTime<Utc>,
}

/// 本文（design D4 の表）。**欄の並びと省略の規則が契約。**
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VisitPayload {
    pub kind: &'static str,
    /// 出来事の時刻（マイクロ秒。D5）。`event_time` と同じ値
    pub at: String,
    pub browser: &'static str,
    pub family: &'static str,
    /// ディレクトリ名。**表示名は載せない**（改名で全訪問が「変わった」にならないため。D4 の注）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visit_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visit_time_raw: Option<i64>,
    /// DB の文字列そのまま。補正しない。URL の行が無い訪問では題名とともに省く（deep.md 第 4 回 Q8）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// 題名が NULL なら欄を省く
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<i64>,
    /// 下位 8 ビット（Firefox は `visit_type`）の名前。知らない値なら省く
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition_core: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_visit: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opener_visit: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visit_duration_us: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub originator_cache_guid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub originator_visit_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_known_to_sync: Option<bool>,
    pub tz_basis: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vanished: Option<Vec<VanishedVisit>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excluded_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profiles: Option<BTreeMap<String, ProfileName>>,
}

/// `profiles` の 1 行。表示名が読めないディレクトリは名前を省く（design D1）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProfileName {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// 出来事の時刻はマイクロ秒（design D5）。ST07 の `contract::rfc3339`（ミリ秒）は流用しない。
pub fn micros(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Micros, true)
}

/// `\x1f` で繋いだ組の SHA-256（16 進 64 桁）。
fn digest(parts: &[&str]) -> String {
    format!("{:x}", Sha256::digest(parts.join("\x1f").as_bytes()))
}

/// Chromium の遷移の下位 8 ビットの名前（`ui/base/page_transition_types.h`）。
fn chromium_core(transition: i64) -> Option<&'static str> {
    Some(match transition & 0xff {
        0 => "link",
        1 => "typed",
        2 => "auto_bookmark",
        3 => "auto_subframe",
        4 => "manual_subframe",
        5 => "generated",
        6 => "auto_toplevel",
        7 => "form_submit",
        8 => "reload",
        9 => "keyword",
        10 => "keyword_generated",
        _ => return None,
    })
}

/// Firefox の `visit_type` の名前（`nsINavHistoryService`）。
fn firefox_core(visit_type: i64) -> Option<&'static str> {
    Some(match visit_type {
        1 => "link",
        2 => "typed",
        3 => "bookmark",
        4 => "embed",
        5 => "redirect_permanent",
        6 => "redirect_temporary",
        7 => "download",
        8 => "framed_link",
        9 => "reload",
        _ => return None,
    })
}

impl VisitPayload {
    fn base(kind: &'static str, at: DateTime<Utc>, browser: Browser) -> Self {
        Self {
            kind,
            at: micros(at),
            browser: browser.name(),
            family: browser.family(),
            profile_dir: None,
            visit_id: None,
            visit_time_raw: None,
            url: None,
            title: None,
            transition: None,
            transition_core: None,
            from_visit: None,
            opener_visit: None,
            visit_duration_us: None,
            originator_cache_guid: None,
            originator_visit_id: None,
            is_known_to_sync: None,
            tz_basis: TZ_BASIS,
            vanished: None,
            excluded_count: None,
            profiles: None,
        }
    }
}

impl Visit {
    fn build(
        external_id: String,
        payload: VisitPayload,
        at: DateTime<Utc>,
    ) -> anyhow::Result<Self> {
        let raw = serde_json::to_string(&payload).context("履歴の本文を直列化できない")?;
        Ok(Self {
            external_id,
            payload,
            raw,
            at,
        })
    }

    /// 訪問 1 件（design D4）。識別子は
    /// `v1:visit:<sha256(family \x1f browser \x1f profile_dir \x1f visit_id \x1f visit_time_raw \x1f url)>`（D6）。
    /// **URL の行が無い訪問は組から URL を省く**（5 つの組。空の URL の 6 つの組とも別の値になる。deep.md 第 4 回 Q8）。
    pub fn from_read(browser: Browser, profile_dir: &str, v: &ReadVisit) -> anyhow::Result<Self> {
        let mut p = VisitPayload::base("visit", v.at, browser);
        p.profile_dir = Some(profile_dir.to_owned());
        p.visit_id = Some(v.id);
        p.visit_time_raw = Some(v.visit_time_raw);
        p.url = v.url.clone();
        // 題名は URL の行にあるので、URL の無い訪問は題名も持たない
        p.title = v.url.as_ref().and(v.title.clone());
        p.transition = Some(v.transition);
        p.transition_core = match browser {
            Browser::Firefox => firefox_core(v.transition),
            _ => chromium_core(v.transition),
        };
        p.from_visit = v.from_visit;
        p.opener_visit = v.opener_visit;
        if browser != Browser::Firefox {
            p.visit_duration_us = v.duration_us;
            p.is_known_to_sync = v.is_known_to_sync;
        }
        // 発生元の印は他端末の訪問だけ（空の guid は PC 自身。D9）
        p.originator_cache_guid = v.originator_cache_guid.clone().filter(|g| !g.is_empty());
        p.originator_visit_id = p.originator_cache_guid.as_ref().and(v.originator_visit_id);
        let (visit_id, visit_time_raw) = (v.id.to_string(), v.visit_time_raw.to_string());
        let mut parts = vec![
            browser.family(),
            browser.name(),
            profile_dir,
            &visit_id,
            &visit_time_raw,
        ];
        parts.extend(v.url.as_deref());
        let id = format!("v1:visit:{}", digest(&parts));
        Self::build(id, p, v.at)
    }

    /// 消えた訪問のまとまり 1 件（design D10）。識別子は
    /// `v1:vanished:<sha256(browser \x1f profile_dir \x1f 識別子を整列したもの)>`（D6）——
    /// 取得時刻を入れないので、やり直しても同じ識別子になる。
    pub fn vanished(
        browser: Browser,
        profile_dir: &str,
        now: DateTime<Utc>,
        mut items: Vec<VanishedVisit>,
    ) -> anyhow::Result<Self> {
        items.sort_by(|a, b| a.external_id.cmp(&b.external_id));
        let mut parts = vec![browser.name(), profile_dir];
        parts.extend(items.iter().map(|i| i.external_id.as_str()));
        let id = format!("v1:vanished:{}", digest(&parts));
        let mut p = VisitPayload::base("vanished", now, browser);
        p.profile_dir = Some(profile_dir.to_owned());
        p.vanished = Some(items);
        Self::build(id, p, now)
    }

    /// その回に新しく除外した訪問の数 1 件（design D11）。識別子は
    /// `v1:excluded:<sha256(browser \x1f profile_dir \x1f 新しく除外した訪問の識別子を整列したもの)>`（D6）。
    pub fn excluded(
        browser: Browser,
        profile_dir: &str,
        now: DateTime<Utc>,
        newly_excluded: &[String],
    ) -> anyhow::Result<Self> {
        let mut ids: Vec<&str> = newly_excluded.iter().map(String::as_str).collect();
        ids.sort_unstable();
        let mut parts = vec![browser.name(), profile_dir];
        parts.extend(ids);
        let id = format!("v1:excluded:{}", digest(&parts));
        let mut p = VisitPayload::base("excluded", now, browser);
        p.profile_dir = Some(profile_dir.to_owned());
        p.excluded_count = Some(newly_excluded.len());
        Self::build(id, p, now)
    }

    /// ディレクトリ名 → 表示名の対応 1 件（design D1）。識別子は
    /// `v1:profiles:<sha256(browser \x1f 対応表を整列したもの)>`（D6）。対応表の 1 行は
    /// `ディレクトリ名 \x1e 表示名`（表示名が無ければディレクトリ名だけ）。
    pub fn profiles(
        browser: Browser,
        now: DateTime<Utc>,
        mapping: &BTreeMap<String, Option<String>>,
    ) -> anyhow::Result<Self> {
        let rows: Vec<String> = mapping
            .iter()
            .map(|(dir, name)| match name {
                Some(n) => format!("{dir}\x1e{n}"),
                None => dir.clone(),
            })
            .collect();
        let mut parts = vec![browser.name()];
        parts.extend(rows.iter().map(String::as_str));
        let id = format!("v1:profiles:{}", digest(&parts));
        let mut p = VisitPayload::base("profiles", now, browser);
        p.profiles = Some(
            mapping
                .iter()
                .map(|(dir, name)| (dir.clone(), ProfileName { name: name.clone() }))
                .collect(),
        );
        Self::build(id, p, now)
    }
}

/// 他のモジュールのテストが使う Chromium の訪問（時刻は 1601 年起点のマイクロ秒に戻して持たせる）。
#[cfg(test)]
pub(crate) fn sample_visit(id: i64, at: DateTime<Utc>, url: &str, title: &str) -> Visit {
    let epoch = DateTime::parse_from_rfc3339("1601-01-01T00:00:00Z")
        .expect("epoch")
        .with_timezone(&Utc);
    let raw = (at - epoch).num_microseconds().expect("範囲内");
    let read = ReadVisit {
        id,
        visit_time_raw: raw,
        at,
        url: Some(url.into()),
        title: Some(title.into()),
        transition: 0,
        from_visit: None,
        opener_visit: None,
        duration_us: None,
        originator_cache_guid: None,
        originator_visit_id: None,
        is_known_to_sync: None,
    };
    Visit::from_read(Browser::Chrome, "Default", &read).expect("直列化できる")
}

impl IngestRequest {
    /// 履歴 1 件を契約の形にする（design D13。ウィンドウの `of` とは別の組み立て）。
    ///
    /// `source_updated_at` には**その内容を読んだ時刻**を載せる（design D15）。履歴 DB は更新時刻を持たないが、
    /// 読んだ時刻は同じ訪問について単調に進む版として働き、未送信の再送が新しい題名を書き戻すのを防ぐ。
    pub fn of_visit(
        visit: &Visit,
        user_id: uuid::Uuid,
        device_id: &str,
        collected_at: DateTime<Utc>,
        zone: &crate::config::Zone,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            id: uuid::Uuid::new_v4(),
            user_id,
            logical_source: LOGICAL_SOURCE.to_string(),
            external_id: Some(visit.external_id.clone()),
            device_id: device_id.to_string(),
            origin: "collected".to_string(),
            event_time: visit.payload.at.clone(),
            tz_offset_min: zone.offset_min,
            tz_id: zone.id.clone(),
            schema_version: crate::contract::SCHEMA_VERSION,
            source_updated_at: Some(micros(collected_at)),
            raw: visit.raw.clone(),
            payload: serde_json::to_value(&visit.payload)?,
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::history::read::{chromium_micros, firefox_micros};

    fn at() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-09-13T01:02:03.456789Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn read_visit(id: i64, raw: i64, url: &str, title: Option<&str>) -> ReadVisit {
        ReadVisit {
            id,
            visit_time_raw: raw,
            at: chromium_micros(raw).unwrap(),
            url: Some(url.into()),
            title: title.map(Into::into),
            transition: 805_306_368,
            from_visit: Some(3),
            opener_visit: None,
            duration_us: Some(1_500_000),
            originator_cache_guid: None,
            originator_visit_id: None,
            is_known_to_sync: Some(false),
        }
    }

    fn item(id: &str) -> VanishedVisit {
        VanishedVisit {
            external_id: id.into(),
            age_days: 91,
            foreign: true,
            table_recreated: false,
            profile_gone: false,
        }
    }

    #[test]
    fn visit_time_keeps_micros() {
        // Scenario: 訪問時刻がマイクロ秒で残る
        // 13_402_627_200_000_001 µs（1601 年起点）の端数 1 µs が出来事の時刻まで残る
        let v = Visit::from_read(
            Browser::Chrome,
            "Default",
            &read_visit(7, 13_402_627_200_000_001, "https://example.test/a", None),
        )
        .unwrap();
        assert_eq!(v.payload.at, "2025-09-18T00:00:00.000001Z");
        let req = IngestRequest::of_visit(
            &v,
            uuid::Uuid::nil(),
            "pc",
            at(),
            &crate::config::Zone {
                id: "Asia/Tokyo".into(),
                offset_min: 540,
            },
        )
        .unwrap();
        assert_eq!(req.event_time, "2025-09-18T00:00:00.000001Z");
    }

    /// 識別子の式を**独立に計算した値**で固定する（design D6）。期待値は
    /// `printf 'chromium\x1fchrome\x1fDefault\x1f7\x1f13402627200000001\x1fhttps://example.test/a?q=x' | sha256sum`。
    #[test]
    fn visit_external_id_is_pinned() {
        // Scenario: 識別子から URL と訪問時刻とプロファイルが読み取れない
        let url = "https://example.test/a?q=x";
        let v = Visit::from_read(
            Browser::Chrome,
            "Default",
            &read_visit(7, 13_402_627_200_000_001, url, Some("題名")),
        )
        .unwrap();
        assert_eq!(
            v.external_id,
            "v1:visit:981e93fe1ba0a08cc7db349da36b0a824d98bed3037ed77a2722e19479ecefa1"
        );
        let other = Visit::from_read(
            Browser::Chrome,
            "Default",
            &read_visit(8, 13_402_627_260_000_000, url, Some("題名")),
        )
        .unwrap();
        for id in [&v.external_id, &other.external_id] {
            for private in ["example", "Default", "13402627", "2025"] {
                assert!(!id.contains(private), "識別子に {private} が見える: {id}");
            }
        }
        // 同じ URL の別の訪問と、接頭辞のほかに共通する部分（同じ位置の 8 桁）を持たない
        let a = v.external_id.trim_start_matches("v1:visit:");
        let b = other.external_id.trim_start_matches("v1:visit:");
        assert_eq!((a.len(), b.len()), (64, 64));
        assert!(
            (0..=56).all(|i| a[i..i + 8] != b[i..i + 8]),
            "同じ URL の 2 つの識別子が共通の部分を持つ: {a} / {b}"
        );

        // vanished / excluded / profiles は中身から決まる（取得の時刻を入れない）
        let same = "1803175b9956215853eb64167cda3e8457827f37505b0e6e58a6423d7fd469d8";
        let gone = Visit::vanished(
            Browser::Chrome,
            "Default",
            at(),
            vec![item("v1:visit:b"), item("v1:visit:a")],
        )
        .unwrap();
        assert_eq!(gone.external_id, format!("v1:vanished:{same}"));
        let later = at() + chrono::Duration::hours(1);
        let gone_again = Visit::vanished(
            Browser::Chrome,
            "Default",
            later,
            vec![item("v1:visit:a"), item("v1:visit:b")],
        )
        .unwrap();
        assert_eq!(gone_again.external_id, gone.external_id);
        let ex = Visit::excluded(
            Browser::Chrome,
            "Default",
            at(),
            &["v1:visit:b".into(), "v1:visit:a".into()],
        )
        .unwrap();
        assert_eq!(ex.external_id, format!("v1:excluded:{same}"));
        let names = BTreeMap::from([
            ("Default".to_string(), Some("個人".to_string())),
            ("Profile 1".to_string(), None),
        ]);
        let pr = Visit::profiles(Browser::Chrome, at(), &names).unwrap();
        assert_eq!(
            pr.external_id,
            "v1:profiles:5c4f485246b6988da1511f50e3507d49ae5f85204d603f6782b8c72174b8e8db"
        );
    }

    /// URL の行が無い訪問は、URL と題名を省いた `visit` になる。識別子は組から URL を省いたもの（deep.md 第 4 回 Q8。R57）。
    /// 期待値は `printf 'chromium\x1fchrome\x1fDefault\x1f7\x1f13402627200000001' | sha256sum`。
    #[test]
    fn visit_without_url_row_omits_url_and_title() {
        let mut rv = read_visit(7, 13_402_627_200_000_001, "", Some("残っていた題名"));
        rv.url = None;
        let v = Visit::from_read(Browser::Chrome, "Default", &rv).unwrap();
        assert_eq!(
            v.raw,
            r#"{"kind":"visit","at":"2025-09-18T00:00:00.000001Z","browser":"chrome","family":"chromium","profile_dir":"Default","visit_id":7,"visit_time_raw":13402627200000001,"transition":805306368,"transition_core":"link","from_visit":3,"visit_duration_us":1500000,"is_known_to_sync":false,"tz_basis":"collected-at"}"#
        );
        assert_eq!(
            v.external_id,
            "v1:visit:c8aa63ce0ee01af0c4bcd0044f6feacd57ee01983151c3ab96189c456acd5d7a"
        );
        // 空の URL の訪問とは別の識別子
        let empty = Visit::from_read(
            Browser::Chrome,
            "Default",
            &read_visit(7, 13_402_627_200_000_001, "", None),
        )
        .unwrap();
        assert_ne!(empty.external_id, v.external_id);
    }

    #[test]
    fn visit_payload_shape_is_pinned() {
        // Scenario: タイムゾーンが取得時のものだと本文から分かる
        // Scenario: 記録の本文にブラウザとプロファイルがある
        let mut rv = read_visit(
            7,
            13_402_627_200_000_001,
            "https://example.test/a",
            Some("題名"),
        );
        rv.opener_visit = Some(5);
        let v = Visit::from_read(Browser::Chrome, "Default", &rv).unwrap();
        assert_eq!(
            v.raw,
            r#"{"kind":"visit","at":"2025-09-18T00:00:00.000001Z","browser":"chrome","family":"chromium","profile_dir":"Default","visit_id":7,"visit_time_raw":13402627200000001,"url":"https://example.test/a","title":"題名","transition":805306368,"transition_core":"link","from_visit":3,"opener_visit":5,"visit_duration_us":1500000,"is_known_to_sync":false,"tz_basis":"collected-at"}"#
        );
        assert_eq!(v.raw, serde_json::to_string(&v.payload).unwrap());

        // 題名が NULL なら欄ごと省く。Firefox は滞在時間と同期の印を持たない
        let mut ff = read_visit(1, 1_758_153_600_000_000, "https://example.test/b", None);
        ff.at = firefox_micros(1_758_153_600_000_000).unwrap();
        ff.transition = 2;
        ff.from_visit = None;
        let f = Visit::from_read(Browser::Firefox, "abc.default", &ff).unwrap();
        assert_eq!(
            f.raw,
            r#"{"kind":"visit","at":"2025-09-18T00:00:00.000000Z","browser":"firefox","family":"firefox","profile_dir":"abc.default","visit_id":1,"visit_time_raw":1758153600000000,"url":"https://example.test/b","transition":2,"transition_core":"typed","tz_basis":"collected-at"}"#
        );

        // 他の端末の訪問だけ発生元の印を持つ。識別子は読んだ PC 側の番号で決まる
        let local = read_visit(1, 13_402_627_200_000_000, "https://a", Some("a"));
        let mut foreign = local.clone();
        foreign.originator_cache_guid = Some("other-pc".into());
        foreign.originator_visit_id = Some(99);
        let l = Visit::from_read(Browser::Chrome, "Default", &local).unwrap();
        let o = Visit::from_read(Browser::Chrome, "Default", &foreign).unwrap();
        assert!(!l.raw.contains("originator"), "{}", l.raw);
        assert!(o
            .raw
            .contains(r#""originator_cache_guid":"other-pc","originator_visit_id":99"#));
        assert_eq!(o.external_id, l.external_id);

        let gone =
            Visit::vanished(Browser::Edge, "Default", at(), vec![item("v1:visit:x")]).unwrap();
        assert_eq!(
            gone.raw,
            r#"{"kind":"vanished","at":"2026-09-13T01:02:03.456789Z","browser":"edge","family":"chromium","profile_dir":"Default","tz_basis":"collected-at","vanished":[{"external_id":"v1:visit:x","age_days":91,"foreign":true,"table_recreated":false,"profile_gone":false}]}"#
        );
        let ex = Visit::excluded(Browser::Chrome, "Default", at(), &["v1:visit:x".into()]).unwrap();
        assert_eq!(
            ex.raw,
            r#"{"kind":"excluded","at":"2026-09-13T01:02:03.456789Z","browser":"chrome","family":"chromium","profile_dir":"Default","tz_basis":"collected-at","excluded_count":1}"#
        );
        let names = BTreeMap::from([
            ("Default".to_string(), Some("個人".to_string())),
            ("Profile 1".to_string(), None),
        ]);
        let pr = Visit::profiles(Browser::Chrome, at(), &names).unwrap();
        assert_eq!(
            pr.raw,
            r#"{"kind":"profiles","at":"2026-09-13T01:02:03.456789Z","browser":"chrome","family":"chromium","tz_basis":"collected-at","profiles":{"Default":{"name":"個人"},"Profile 1":{}}}"#
        );

        // 要求の組み立て: 識別子は external_id に、読んだ時刻は source_updated_at に載る（D15）
        let zone = crate::config::Zone {
            id: "Asia/Tokyo".into(),
            offset_min: 540,
        };
        let req = IngestRequest::of_visit(&o, uuid::Uuid::nil(), "reader-pc", at(), &zone).unwrap();
        assert_eq!(req.logical_source, LOGICAL_SOURCE);
        assert_eq!(req.external_id.as_deref(), Some(o.external_id.as_str()));
        assert_eq!(req.device_id, "reader-pc");
        assert_eq!(req.raw, o.raw);
        assert_eq!(req.payload["originator_cache_guid"], "other-pc");
        assert_eq!(
            req.source_updated_at.as_deref(),
            Some("2026-09-13T01:02:03.456789Z")
        );
    }
}
