// SPDX-License-Identifier: AGPL-3.0-only
//! 送る形。**`docs/collector-contract.md` と `crates/server/src/ingest.rs` が正典**で、
//! ここはその形を Rust の型に写しただけ。片方だけ直すと同じ 1 件が別物として入る。
//!
//! # 直列化の形を day one で固定する（design D1）
//!
//! 冪等キーは `logical_source` + `event_time` + **`raw` の文字列そのもの**から作られる。
//! **形を後から変えると同じ 1 件が別の鍵になる**ので、`payload_shape_is_pinned` が
//! 1 文字単位で固定する。項目の並び・省略の規則・時刻の刻みまでが契約。
use serde::{Deserialize, Serialize};

use crate::LOGICAL_SOURCE;

/// エンベロープの版（FR-26）。
pub const SCHEMA_VERSION: i32 = 1;

/// 記録の種類。**`core.event` の 1 件がどの出来事なのか**（design D1）。
///
/// `core.coverage_span.kind`（`stopped` / `dropped`）とは**別の語彙**にしてある ——
/// 電源断を `stopped`（＝本人が意図して止めた）に混ぜると扉 #14 の区別を自分で壊す
/// （design D10）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecordKind {
    /// 前景の変化（FR-12）
    Foreground,
    /// 入力が無い状態への出入り（FR-81）
    Idle,
    /// PC が止まっていた期間（FR-82）
    PoweredOff,
    /// 除外された対象（FR-83）。**本文を持たない**
    Excluded,
    /// 時計のずれの測定（FR-7 / 扉 #5 / design D6）
    ClockSkew,
}

impl RecordKind {
    /// 送る文字列。**`core.coverage_span.kind` の語彙と重ならない**ことを
    /// `powered_off_is_not_stopped` が固定する。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Foreground => "foreground",
            Self::Idle => "idle",
            Self::PoweredOff => "powered-off",
            Self::Excluded => "excluded",
            Self::ClockSkew => "clock-skew",
        }
    }
}

/// 入力が無い状態への出入りのどちら側か（design D14）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Transition {
    /// 入力が無い状態に入った
    Enter,
    /// そこから出た。**この記録が区間（`range_end`）を持つ**
    Leave,
}

/// 入力が無かった理由（FR-81 の「離席・画面ロック・スリープ」）。
///
/// **区別して残す。** 後から「離席だけを数えたい」となったときに、
/// 理由を残していなければ引き直せない（取っていないものは作れない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AwayReason {
    /// 入力が閾値（design D9）を超えて無かった
    Idle,
    /// 画面がロックされていた
    Locked,
    /// PC が眠っていた（見回りの時刻が飛んだ。design D19）
    Suspended,
}

/// 入力が無い区間が**入力の再開以外で**閉じられた理由（design D14 / D21）。
///
/// 入力が戻って閉じた普通の区間は持たない（欄を省く）。**閉じ方が違う区間を
/// 同じ形で残すと、「いつ戻ったか」として読まれてしまう。**
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EndedBy {
    /// 別の理由の区間に入れ替わった（離席のまま画面がロックされた、など）
    Superseded,
    /// 収集が止まった（`range_end` は最後に動いていた時刻）
    Restart,
    /// 入力の経過時間が読めなくなった（`range_end` は最後に読めた時刻）
    Unreadable,
}

/// 記録 1 件の中身。**`raw` と `payload` の両方がこの形**（design D1）。
///
/// `raw` は取り込み口が素通しで残し、`payload` は文字列が NFC に揃えられる。
/// SQL から引けるのは `payload` だけ（`raw` は `text`）なので、
/// **FR-58 が名指しする「アプリ名・ウィンドウ題名・URL」は `payload` の項目に持つ**。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowPayload {
    /// 記録の種類（design D1）
    pub kind: RecordKind,
    /// 出来事が起きた時刻。**`event_time` と同じ値を原文にも入れる。**
    ///
    /// 入れないと、本文を持たない記録（`excluded`）の原文が全部同じ文字列になり、
    /// **冪等キー（`logical_source` + `event_time` + `raw`）が同じ秒の 2 件を
    /// 1 件に畳む** —— 除外の件数が黙って減る。
    pub at: String,
    /// アプリの表示名。**除外された記録では持たない**（FR-83）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_name: Option<String>,
    /// 実行ファイルのパス。OS から費用ゼロで取れるので入れる側に倒す（design D1）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exe_path: Option<String>,
    /// プロセス名。同上
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_name: Option<String>,
    /// ウィンドウ題名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// アドレスバーに**見えている文字列そのまま**（深掘り Q4 / design D12）。
    /// 補正しない —— `https://` を補わず、省略を展開しない
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// 前景はブラウザだが**アドレスバーが読めなかった**（design D4 / R25）。
    /// `url` が無い理由を「ブラウザではない」と区別するために残す。
    /// 読めた記録では省く
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_unavailable: Option<bool>,
    /// 範囲の終わり。`idle`（`leave` 側）と `powered-off` だけが持つ
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range_end: Option<String>,
    /// 最後の入力からの経過時間（`idle` のみ。design D9）。
    /// **閾値を後から変えたときに引き直せるようにするため**に載せる
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_ms: Option<i64>,
    /// 入りと出のどちら側か（`idle` のみ。design D14）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<Transition>,
    /// 入力が無かった理由（`idle` のみ）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<AwayReason>,
    /// 入力の再開**以外で**閉じた区間の、閉じ方（`idle` の出た側のみ）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_by: Option<EndedBy>,
    /// 見回りが飛んだ間に**単調時計**が進んだ長さ（`suspended` のみ。design D19）。
    /// 壁時計の飛びと比べれば、「止まっていた」と「時計だけが進んだ」を後から分けられる
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mono_gap_ms: Option<i64>,
    /// 除外した変化の件数（`excluded` のみ。FR-83 / design D11 / D18）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excluded_count: Option<u32>,
    /// OS が最後に起動した時刻（`powered-off` のみ。design D23）。
    /// **区間の始まりより後なら PC は本当に止まっていた。前なら PC は動いていて
    /// 収集だけが止まっていた**（深掘り Q8 の「効く先」を成り立たせる材料）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boot_at: Option<String>,
    /// 前回の収集が**自分で止まった**か（`powered-off` のみ。design D23）。
    /// 止まる前に書く印があれば `true`、無ければ省く（電源断・強制終了・異常終了）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clean_stop: Option<bool>,
    /// 基準時刻との差（`clock-skew` のみ。正なら PC の時計が進んでいる。design D17）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skew_ms: Option<i64>,
    /// ずれを測った基準の出どころ（`clock-skew` のみ。design D17 / R15）。
    /// **取り込み口の `host:port`** —— ループバックなら「自分の時計と比べた 0」だと後から分かる
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skew_reference: Option<String>,
}

impl WindowPayload {
    /// 種類と時刻だけを持つ土台。**本文の項目は呼ぶ側が足す。**
    ///
    /// 既定を「本文なし」にしてあるのは、除外（FR-83）で**足し忘れではなく
    /// 入れ忘れが安全側**になるようにするため。
    pub fn new(kind: RecordKind, at: chrono::DateTime<chrono::Utc>) -> Self {
        Self {
            kind,
            at: rfc3339(at),
            app_name: None,
            exe_path: None,
            process_name: None,
            title: None,
            url: None,
            url_unavailable: None,
            range_end: None,
            idle_ms: None,
            transition: None,
            reason: None,
            ended_by: None,
            mono_gap_ms: None,
            excluded_count: None,
            boot_at: None,
            clean_stop: None,
            skew_ms: None,
            skew_reference: None,
        }
    }
}

/// 記録の時刻の書き方。**ミリ秒まで・UTC・`Z` 終わり**に固定する。
///
/// 刻みを変えると同じ 1 件が別の鍵になる（design D1）。秒で切ると、
/// 同じ秒に起きた前景の変化が 1 件に畳まれる（切り替えは 1 日数千件ある）。
pub fn rfc3339(at: chrono::DateTime<chrono::Utc>) -> String {
    at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// 取り込み口へ送る 1 件（`POST /ingest`）。
///
/// **`sensitivity` の欄を持たない。** 深掘り Q3 で本人が
/// 「PERM-3 のまま（外部 AI に出してよい）」を選んでいる（8 問のうち唯一、
/// 推奨と違う側）。**収集側が厳しい側を付けると QS-7 / QS-10 が答えられなくなり、
/// 倒したことは誰も気付かない。**
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IngestRequest {
    /// 収集側で毎回新しく振る（深掘り Q14）。冪等キーには混ざらない
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub logical_source: String,
    /// **常に `null` を送る。** 空文字は `empty_external_id` で断られる（ST03 / R12）
    pub external_id: Option<String>,
    /// `origin` が `collected` なら必須（FR-24）
    pub device_id: String,
    pub origin: String,
    pub event_time: String,
    pub tz_offset_min: i32,
    pub tz_id: String,
    pub schema_version: i32,
    /// 取得元の内容を読んだ時刻。履歴の再送で新しい版を書き戻さないために使う。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_updated_at: Option<String>,
    /// 原文。**収集側が組んだ JSON を文字列のまま**送る（design D1）
    pub raw: String,
    /// 解析済み。SQL から引けるのはこちら
    pub payload: serde_json::Value,
}

impl IngestRequest {
    /// 記録 1 件を契約の形にする。
    ///
    /// **`raw` と `payload` は同じ中身から作る** —— 別々に組むと、
    /// 原文と解析済みが食い違ったまま凍結される（移行 0004 のトリガ）。
    pub fn of(
        payload: &WindowPayload,
        user_id: uuid::Uuid,
        device_id: &str,
        at: chrono::DateTime<chrono::Utc>,
        zone: &crate::config::Zone,
    ) -> anyhow::Result<Self> {
        let raw = serde_json::to_string(payload)?;
        Ok(Self {
            id: uuid::Uuid::new_v4(),
            user_id,
            logical_source: LOGICAL_SOURCE.to_string(),
            external_id: None,
            device_id: device_id.to_string(),
            origin: "collected".to_string(),
            event_time: rfc3339(at),
            tz_offset_min: zone.offset_min,
            tz_id: zone.id.clone(),
            schema_version: SCHEMA_VERSION,
            source_updated_at: None,
            raw,
            payload: serde_json::to_value(payload)?,
        })
    }

    /// ブラウザ履歴の訪問を、訪問ごとの識別子を持つ要求へ変換する。
    pub fn of_visit(
        visit: &crate::history::contract::Visit,
        user_id: uuid::Uuid,
        device_id: &str,
        collected_at: chrono::DateTime<chrono::Utc>,
        zone: &crate::config::Zone,
    ) -> anyhow::Result<Self> {
        let raw = serde_json::to_string(&visit.payload)?;
        Ok(Self {
            id: uuid::Uuid::new_v4(),
            user_id,
            logical_source: "c02-browser-history".to_string(),
            external_id: Some(visit.external_id.clone()),
            device_id: device_id.to_string(),
            origin: "collected".to_string(),
            event_time: visit.payload.at.clone(),
            tz_offset_min: zone.offset_min,
            tz_id: zone.id.clone(),
            schema_version: SCHEMA_VERSION,
            source_updated_at: Some(rfc3339(collected_at)),
            raw,
            payload: serde_json::to_value(&visit.payload)?,
        })
    }
}

/// 生存信号 1 件（`POST /heartbeat`）。**記録とは別の受け口**（ST02 design D9）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeartbeatRequest {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub logical_source: String,
    pub device_id: String,
    /// **信号を作った時刻。** 受信時刻ではない
    pub emitted_at: String,
    /// そのソースを取得できる状態か（design D4）
    pub capturable: bool,
    /// 満たされていないものの名前。**`capturable = false` なら空にできない**
    pub blockers: Vec<String>,
    /// 前回の信号からの取得の試行回数
    pub attempts: i32,
    /// そのうち成功した回数。**試行を超えない**（超えると受け口が断る）
    pub successes: i32,
    pub raw: String,
}

/// 未送信に積めるもの。**識別子で取り除く**ので、それだけを要求する。
pub trait Outboxable: Serialize + serde::de::DeserializeOwned + Clone {
    /// 収集側が振った識別子。
    fn id(&self) -> uuid::Uuid;
}

impl Outboxable for IngestRequest {
    fn id(&self) -> uuid::Uuid {
        self.id
    }
}

impl Outboxable for HeartbeatRequest {
    fn id(&self) -> uuid::Uuid {
        self.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Zone;

    fn at() -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339("2026-09-13T01:02:03.456Z")
            .expect("固定の時刻")
            .with_timezone(&chrono::Utc)
    }

    fn zone() -> Zone {
        Zone {
            id: "Asia/Tokyo".into(),
            offset_min: 540,
        }
    }

    /// **直列化の形を 1 文字単位で固定する**（tasks 6.3 / design D1）。
    ///
    /// 形が変わると、**同じ 1 件が別の鍵になって二重に入る**。
    /// 4 章・6 章で項目が出そろってから凍結した（spec-review R6）。
    #[test]
    fn payload_shape_is_pinned() {
        let mut fg = WindowPayload::new(RecordKind::Foreground, at());
        fg.app_name = Some("ブラウザ".into());
        fg.exe_path = Some(r"C:\app\b.exe".into());
        fg.process_name = Some("b.exe".into());
        fg.title = Some("題名".into());
        fg.url = Some("example.com/a?q=1#f".into());
        assert_eq!(
            serde_json::to_string(&fg).expect("直列化"),
            r#"{"kind":"foreground","at":"2026-09-13T01:02:03.456Z","app_name":"ブラウザ","exe_path":"C:\\app\\b.exe","process_name":"b.exe","title":"題名","url":"example.com/a?q=1#f"}"#
        );

        let mut idle = WindowPayload::new(RecordKind::Idle, at());
        idle.range_end = Some(rfc3339(at() + chrono::Duration::minutes(30)));
        idle.idle_ms = Some(1_800_000);
        idle.transition = Some(Transition::Leave);
        idle.reason = Some(AwayReason::Locked);
        assert_eq!(
            serde_json::to_string(&idle).expect("直列化"),
            r#"{"kind":"idle","at":"2026-09-13T01:02:03.456Z","range_end":"2026-09-13T01:32:03.456Z","idle_ms":1800000,"transition":"leave","reason":"locked"}"#
        );

        let mut off = WindowPayload::new(RecordKind::PoweredOff, at());
        off.range_end = Some(rfc3339(at() + chrono::Duration::hours(12)));
        assert_eq!(
            serde_json::to_string(&off).expect("直列化"),
            r#"{"kind":"powered-off","at":"2026-09-13T01:02:03.456Z","range_end":"2026-09-13T13:02:03.456Z"}"#
        );
        off.boot_at = Some(rfc3339(at() + chrono::Duration::hours(11)));
        off.clean_stop = Some(true);
        assert_eq!(
            serde_json::to_string(&off).expect("直列化"),
            r#"{"kind":"powered-off","at":"2026-09-13T01:02:03.456Z","range_end":"2026-09-13T13:02:03.456Z","boot_at":"2026-09-13T12:02:03.456Z","clean_stop":true}"#
        );

        let mut slept = WindowPayload::new(RecordKind::Idle, at());
        slept.range_end = Some(rfc3339(at() + chrono::Duration::hours(1)));
        slept.idle_ms = Some(3_600_000);
        slept.transition = Some(Transition::Leave);
        slept.reason = Some(AwayReason::Suspended);
        slept.ended_by = Some(EndedBy::Superseded);
        slept.mono_gap_ms = Some(1_200);
        assert_eq!(
            serde_json::to_string(&slept).expect("直列化"),
            r#"{"kind":"idle","at":"2026-09-13T01:02:03.456Z","range_end":"2026-09-13T02:02:03.456Z","idle_ms":3600000,"transition":"leave","reason":"suspended","ended_by":"superseded","mono_gap_ms":1200}"#
        );

        let mut blind = WindowPayload::new(RecordKind::Foreground, at());
        blind.app_name = Some("ブラウザ".into());
        blind.url_unavailable = Some(true);
        assert_eq!(
            serde_json::to_string(&blind).expect("直列化"),
            r#"{"kind":"foreground","at":"2026-09-13T01:02:03.456Z","app_name":"ブラウザ","url_unavailable":true}"#
        );

        let mut ex = WindowPayload::new(RecordKind::Excluded, at());
        ex.excluded_count = Some(3);
        assert_eq!(
            serde_json::to_string(&ex).expect("直列化"),
            r#"{"kind":"excluded","at":"2026-09-13T01:02:03.456Z","excluded_count":3}"#,
            "除外の記録に本文が混ざっている（FR-83）"
        );

        let mut skew = WindowPayload::new(RecordKind::ClockSkew, at());
        skew.skew_ms = Some(-1200);
        skew.skew_reference = Some("127.0.0.1:8787".into());
        assert_eq!(
            serde_json::to_string(&skew).expect("直列化"),
            r#"{"kind":"clock-skew","at":"2026-09-13T01:02:03.456Z","skew_ms":-1200,"skew_reference":"127.0.0.1:8787"}"#
        );
    }

    /// 原文と解析済みが**同じ中身**から作られる（design D1）。
    #[test]
    fn raw_and_payload_come_from_the_same_value() {
        let mut p = WindowPayload::new(RecordKind::Foreground, at());
        p.title = Some("題名".into());
        let req =
            IngestRequest::of(&p, uuid::Uuid::nil(), "dev-1", at(), &zone()).expect("契約の形");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&req.raw).expect("原文は JSON"),
            req.payload
        );
        assert_eq!(
            req.event_time, p.at,
            "原文の時刻とエンベロープの時刻がずれている"
        );
    }

    /// **収集側は感度を付けない**（深掘り Q3。`sensitivity` の欄そのものを持たない）。
    ///
    /// 「題名と URL は私的だから」と厳しい側に倒すと、成功条件 2 の
    /// QS-7 / QS-10 が day one から答えられなくなる。**倒したことは
    /// 答えられなくなった時にしか判らない**ので、送る本文の側で固定する。
    ///
    /// Scenario: 収集側が厳しい側の感度を付けて送らない
    #[test]
    fn history_sensitivity_uses_collection_default() {
        // Scenario: ブラウザ履歴の記録も既定の感度で格納される
        let visit = crate::history::contract::Visit::new(
            "chrome", "Default", 1, at(), "https://example.test/private", "題名",
        );
        let req = IngestRequest::of_visit(&visit, uuid::Uuid::nil(), "dev-1", at(), &zone())
            .expect("履歴の契約の形");
        let json = serde_json::to_string(&req).expect("直列化");
        assert!(
            !json.contains("sensitivity"),
            "収集側が感度を指定している: {json}"
        );
    }

    /// **`external_id` は `null` で送る。** 空文字は 400（`empty_external_id`）で
    /// 断られる（ST03 / R12。`docs/collector-contract.md` §返る形）。
    #[test]
    fn external_id_is_null_not_empty() {
        let p = WindowPayload::new(RecordKind::Foreground, at());
        let req =
            IngestRequest::of(&p, uuid::Uuid::nil(), "dev-1", at(), &zone()).expect("契約の形");
        let json: serde_json::Value = serde_json::to_value(&req).expect("直列化");
        assert_eq!(json["external_id"], serde_json::Value::Null);
        assert_eq!(json["origin"], "collected");
        assert_eq!(json["logical_source"], LOGICAL_SOURCE);
        assert!(
            json.get("source_updated_at").is_none() && json.get("external_ref").is_none(),
            "省略できる欄を送っている（端末からの収集では常に持たない）"
        );
    }

    /// **記録の種類が稼働記録の語彙と重ならない**（design D10）。
    ///
    /// `powered-off` を `stopped`（＝本人が意図して止めた）に混ぜると、
    /// 扉 #14 が作った区別を自分で壊す。
    ///
    /// Scenario: 意図的な停止と区別できる
    #[test]
    fn powered_off_is_not_stopped() {
        // `core.coverage_span.kind` の語彙（migrations/202609111111_coverage_rebuild.sql）
        let span_kinds = ["stopped", "dropped"];
        for kind in [
            RecordKind::Foreground,
            RecordKind::Idle,
            RecordKind::PoweredOff,
            RecordKind::Excluded,
            RecordKind::ClockSkew,
        ] {
            assert!(
                !span_kinds.contains(&kind.as_str()),
                "記録の種類が稼働記録の語彙と重なっている: {}",
                kind.as_str()
            );
        }
        assert_eq!(RecordKind::PoweredOff.as_str(), "powered-off");
    }
}
