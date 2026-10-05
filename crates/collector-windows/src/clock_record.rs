// SPDX-License-Identifier: AGPL-3.0-only
//! 作業スレッドが読んだ基準を、PC の測定記録（`clock-skew`）にする（ST05 design D10）。
//!
//! 2 つの出どころ（取り込み口の応答の日付・Windows の時刻同期の状態）は、
//! **取れた基準（`clock_references`）か取れなかった基準（`clock_unavailable`）のどちらかに 1 回ずつ**入る。
use chrono::{DateTime, Utc};

use crate::clock_worker::ClockReading;
use crate::contract::{
    rfc3339, ClockReference, ClockTrigger, ClockUnavailable, RecordKind, WindowPayload,
    SOURCE_S01_DATE, SOURCE_TIME_SYNC,
};
use crate::time_sync::raw_text;

/// 測定記録を積む時点の PC の状態。**差に使う壁時計（基準を読んだ直後）とは別**（design D10）。
#[derive(Debug, Clone)]
pub struct RecordContext {
    /// 測った契機
    pub trigger: ClockTrigger,
    /// 測定を積むときの PC の時計（記録の出来事時刻。補正しない）
    pub at: DateTime<Utc>,
    /// 測定を積むときの起動からの経過時間（ミリ秒）
    pub uptime_ms: u64,
    /// OS が最後に起動した時刻（起動の識別。取れなければ `None`）
    pub boot_at: Option<DateTime<Utc>>,
}

/// 読み取りから測定記録を作る。**差は基準を受け取った直後の壁時計で計算済み**（`wall_after`）。
pub fn skew_record(ctx: &RecordContext, reading: ClockReading) -> WindowPayload {
    let mut references = Vec::new();
    let mut unavailable = Vec::new();
    let mut p = WindowPayload::new(RecordKind::ClockSkew, ctx.at);

    match reading.reference {
        Ok(r) => {
            let skew_ms = (r.wall_after - r.time).num_milliseconds();
            // ST07 の読む側のために、取れたときだけ残す
            p.skew_ms = Some(skew_ms);
            p.skew_reference = Some(reading.source.clone());
            references.push(ClockReference {
                source: SOURCE_S01_DATE.into(),
                time: Some(rfc3339(r.time)),
                skew_ms: Some(skew_ms),
                os_offset_ms: None,
                mono_before_ms: r.uptime_before_ms,
                mono_after_ms: r.uptime_after_ms,
                host: Some(reading.source),
                raw: None,
                last_sync: None,
                sync_source: None,
                sync_via: None,
            });
        }
        Err(reason) => unavailable.push(ClockUnavailable {
            source: SOURCE_S01_DATE.into(),
            reason,
            raw: None,
        }),
    }

    match reading.time_sync {
        // 項目を 1 つも読めなかった出力は「取れなかった」に入れ、原文を持たせる
        Ok(t) if t.reading.reason.is_some() => unavailable.push(ClockUnavailable {
            source: SOURCE_TIME_SYNC.into(),
            reason: t.reading.reason.unwrap_or_default(),
            raw: Some(raw_text(&t.reading.raw)),
        }),
        Ok(t) => references.push(ClockReference {
            source: SOURCE_TIME_SYNC.into(),
            time: None,
            skew_ms: None,
            os_offset_ms: t.reading.os_offset_ms,
            mono_before_ms: t.uptime_before_ms,
            mono_after_ms: t.uptime_after_ms,
            host: None,
            raw: Some(raw_text(&t.reading.raw)),
            last_sync: t.reading.last_sync,
            sync_source: t.reading.source,
            sync_via: Some(t.reading.via.as_str().into()),
        }),
        Err(reason) => unavailable.push(ClockUnavailable {
            source: SOURCE_TIME_SYNC.into(),
            reason,
            raw: None,
        }),
    }

    p.clock_trigger = Some(ctx.trigger);
    p.clock_available = Some(!references.is_empty());
    p.uptime_ms = Some(ctx.uptime_ms);
    p.boot_at = ctx.boot_at.map(rfc3339);
    p.clock_references = Some(references);
    p.clock_unavailable = Some(unavailable);
    p
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use chrono::Duration;

    use super::*;
    use crate::clock::ReferenceReading;
    use crate::clock_worker::TimedTimeSync;
    use crate::time_sync::{parse_event_log, parse_status, TimeSyncReading, TimeSyncVia};

    fn t(sec: i64) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-09-13T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
            + Duration::seconds(sec)
    }

    fn ctx() -> RecordContext {
        RecordContext {
            trigger: ClockTrigger::Hourly,
            at: t(3_600),
            uptime_ms: 123_456_000,
            boot_at: Some(t(-86_400)),
        }
    }

    /// 取り込み口の基準（PC の時計が 1.2 秒進んでいる）。
    fn reference() -> Result<ReferenceReading, String> {
        Ok(ReferenceReading {
            time: t(3_600) - Duration::milliseconds(1_200),
            uptime_before_ms: 123_455_000,
            uptime_after_ms: 123_455_999,
            wall_after: t(3_600),
        })
    }

    fn time_sync() -> Result<TimedTimeSync, String> {
        Ok(TimedTimeSync {
            reading: TimeSyncReading {
                raw: b"Source: time.windows.com,0x8\r\n".to_vec(),
                last_sync: Some("9/29/2026 12:34:56 PM".into()),
                source: Some("time.windows.com,0x8".into()),
                os_offset_ms: Some(-1),
                reason: None,
                via: TimeSyncVia::W32tm,
            },
            uptime_before_ms: 123_455_100,
            uptime_after_ms: 123_455_200,
        })
    }

    fn reading(
        reference: Result<ReferenceReading, String>,
        time_sync: Result<TimedTimeSync, String>,
    ) -> ClockReading {
        ClockReading {
            source: "127.0.0.1:8787".into(),
            reference,
            time_sync,
        }
    }

    fn names(v: &[ClockReference]) -> Vec<&str> {
        v.iter().map(|r| r.source.as_str()).collect()
    }

    /// Scenario: PC の測定記録に起動の識別と起動からの経過時間が入っている
    #[test]
    fn clock_skew_payload_has_boot_identity_and_uptime() {
        let p = skew_record(&ctx(), reading(reference(), time_sync()));
        assert_eq!(p.kind, RecordKind::ClockSkew);
        assert_eq!(p.at, rfc3339(t(3_600)), "測ったときの PC の時計");
        assert_eq!(p.uptime_ms, Some(123_456_000));
        assert_eq!(p.boot_at.as_deref(), Some(rfc3339(t(-86_400)).as_str()));
        assert_eq!(p.clock_trigger, Some(ClockTrigger::Hourly));
        assert_eq!(p.clock_available, Some(true));
    }

    /// Scenario: 基準ごとに読む直前と直後の経過時間が入っている
    #[test]
    fn clock_skew_payload_has_uptime_pair_per_reference() {
        let p = skew_record(&ctx(), reading(reference(), time_sync()));
        let refs = p.clock_references.unwrap();
        assert_eq!(names(&refs), [SOURCE_S01_DATE, SOURCE_TIME_SYNC]);
        assert_eq!(
            (refs[0].mono_before_ms, refs[0].mono_after_ms),
            (123_455_000, 123_455_999)
        );
        assert_eq!(
            (refs[1].mono_before_ms, refs[1].mono_after_ms),
            (123_455_100, 123_455_200)
        );
        assert_eq!(refs[0].skew_ms, Some(1_200));
        assert_eq!(refs[0].host.as_deref(), Some("127.0.0.1:8787"));
    }

    /// Scenario: 同じ機械の構成でも Windows の時刻同期の状態が並ぶ
    #[test]
    fn clock_skew_payload_lists_time_sync_beside_loopback_destination() {
        // 取り込み口が同じ機械（ループバック）でも、宛先と時刻同期の状態が同じ記録に並ぶ
        let p = skew_record(&ctx(), reading(reference(), time_sync()));
        assert_eq!(p.skew_reference.as_deref(), Some("127.0.0.1:8787"));
        let refs = p.clock_references.unwrap();
        assert_eq!(refs[0].host.as_deref(), Some("127.0.0.1:8787"));
        let sync = &refs[1];
        assert_eq!(sync.sync_source.as_deref(), Some("time.windows.com,0x8"));
        assert_eq!(sync.last_sync.as_deref(), Some("9/29/2026 12:34:56 PM"));
        assert_eq!(sync.os_offset_ms, Some(-1));
        assert!(sync.raw.is_some(), "読んだままの出力が残る");
        assert_eq!(sync.skew_ms, None, "状態には差を置かない（D8）");
    }

    /// W32Time が止まっていてイベントログから読んだ同期は、取れた基準に `sync_via: eventlog` で並ぶ（deep Q4）。
    ///
    /// Scenario: Windows の時刻同期のサービスが止まっていても最後の同期が並ぶ
    #[test]
    fn clock_skew_payload_marks_time_sync_read_from_the_event_log() {
        let xml = b"<Event><System><TimeCreated SystemTime='2026-09-30T11:47:51.9244260Z'/></System><EventData><Data Name='TimeSource'>time.windows.com,0x9</Data></EventData></Event>".to_vec();
        let from_log = Ok(TimedTimeSync {
            reading: parse_event_log(xml).unwrap(),
            uptime_before_ms: 1,
            uptime_after_ms: 2,
        });
        let p = skew_record(&ctx(), reading(Err("unreachable".into()), from_log));
        let refs = p.clock_references.unwrap();
        assert_eq!(names(&refs), [SOURCE_TIME_SYNC]);
        assert_eq!(refs[0].sync_via.as_deref(), Some("eventlog"));
        assert_eq!(
            refs[0].last_sync.as_deref(),
            Some("2026-09-30T11:47:51.9244260Z")
        );
        assert_eq!(refs[0].sync_source.as_deref(), Some("time.windows.com,0x9"));
        assert!(refs[0].raw.as_deref().unwrap().starts_with("<Event>"));
        assert_eq!(p.clock_available, Some(true));
        // w32tm から読んだものは `w32tm`
        let p = skew_record(&ctx(), reading(reference(), time_sync()));
        assert_eq!(
            p.clock_references.unwrap()[1].sync_via.as_deref(),
            Some("w32tm")
        );
    }

    /// Scenario: 2 つの出どころは取れたか取れなかったかのどちらかに 1 回ずつ出る
    #[test]
    fn clock_skew_payload_places_each_source_exactly_once() {
        let unparsed = || {
            Ok(TimedTimeSync {
                reading: parse_status(b"nothing useful\r\n".to_vec()),
                uptime_before_ms: 1,
                uptime_after_ms: 2,
            })
        };
        let combos = [
            (reference(), time_sync()),
            (reference(), Err("timeout".to_string())),
            (Err("unreachable".to_string()), time_sync()),
            (
                Err("unreachable".to_string()),
                Err("spawn_failed".to_string()),
            ),
            (reference(), unparsed()),
            (Err("worker_failed".to_string()), unparsed()),
        ];
        for (r, s) in combos {
            let p = skew_record(&ctx(), reading(r, s));
            let refs = p.clock_references.unwrap();
            let un = p.clock_unavailable.unwrap();
            for source in [SOURCE_S01_DATE, SOURCE_TIME_SYNC] {
                let n = refs.iter().filter(|x| x.source == source).count()
                    + un.iter().filter(|x| x.source == source).count();
                assert_eq!(n, 1, "{source} が {n} 回: {refs:?} / {un:?}");
            }
            assert_eq!(p.clock_available, Some(!refs.is_empty()));
        }
    }

    /// 項目を読めなかった出力は原文つきで取れなかった側に入る。差は取れたときだけ残る。
    ///
    /// Scenario: 項目を読み取れなかった出力も残る
    #[test]
    fn clock_skew_payload_keeps_unparsed_output_and_omits_skew_when_unavailable() {
        let unparsed = Ok(TimedTimeSync {
            reading: parse_status(b"\x8d\xc5 nothing\r\n".to_vec()),
            uptime_before_ms: 1,
            uptime_after_ms: 2,
        });
        let p = skew_record(&ctx(), reading(Err("timeout".into()), unparsed));
        let un = p.clock_unavailable.unwrap();
        assert_eq!(un[0].reason, "timeout");
        assert_eq!(un[1].reason, "unparsed");
        assert_eq!(un[1].raw.as_deref(), Some("\\x8d\\xc5 nothing\r\n"));
        assert_eq!((p.skew_ms, p.skew_reference), (None, None));
        assert_eq!(p.clock_available, Some(false));
        assert_eq!(p.clock_references, Some(Vec::new()));
    }
}
