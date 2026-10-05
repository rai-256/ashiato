// SPDX-License-Identifier: AGPL-3.0-only
//! **収集側の crate が組んだ本文**を 1 件ぶん標準出力へ出す（`tools/smoke.sh` の手順 37 / 39）。
//!
//! smoke が `jq` で手書きした本文を送っていたので、収集側の組み立て（`IngestRequest::of` /
//! `heartbeat::signal`）は 1 度も本物の取り込み口を通っていなかった（review/code.md R5）。
//!
//! ```text
//! cargo run -q -p ashiato-collector-windows --example sample_body -- ingest
//! cargo run -q -p ashiato-collector-windows --example sample_body -- heartbeat
//! cargo run -q -p ashiato-collector-windows --example sample_body -- clock
//! ```
use std::io::Write as _;

use ashiato_collector_windows::{
    clock::ReferenceReading,
    clock_record::{skew_record, RecordContext},
    clock_worker::{ClockReading, TimedTimeSync},
    config::Zone,
    contract, heartbeat, time_sync,
};

fn main() -> anyhow::Result<()> {
    let at =
        chrono::DateTime::parse_from_rfc3339("2026-03-02T01:00:00Z")?.with_timezone(&chrono::Utc);
    let user = uuid::Uuid::nil();
    let body = match std::env::args().nth(1).as_deref() {
        Some("heartbeat") => {
            let cap = heartbeat::Capability::of(true, false);
            serde_json::to_string(&[heartbeat::signal(user, "pc-01", at, &cap, 300, 120)?])?
        }
        Some("clock") => {
            // 取り込み口の基準（PC の時計が 1.2 秒進んでいる）と Windows の時刻同期の状態が両方取れた測定記録
            let ctx = RecordContext {
                trigger: contract::ClockTrigger::Hourly,
                at,
                uptime_ms: 123_456_000,
                boot_at: Some(at - chrono::Duration::days(1)),
            };
            let reading = ClockReading {
                source: "127.0.0.1:8787".into(),
                reference: Ok(ReferenceReading {
                    time: at - chrono::Duration::milliseconds(1_200),
                    uptime_before_ms: 123_455_000,
                    uptime_after_ms: 123_455_999,
                    wall_after: at,
                }),
                time_sync: Ok(TimedTimeSync {
                    reading: time_sync::parse_status(
                        b"Last Successful Sync Time: 9/29/2026 12:34:56 PM\r\nSource: time.windows.com,0x8\r\nPhase Offset: 0.0004680s\r\n".to_vec(),
                    ),
                    uptime_before_ms: 123_455_100,
                    uptime_after_ms: 123_455_200,
                }),
            };
            let p = skew_record(&ctx, reading);
            let zone = Zone {
                id: "Asia/Tokyo".into(),
                offset_min: 540,
            };
            serde_json::to_string(&[contract::IngestRequest::of(&p, user, "pc-01", at, &zone)?])?
        }
        _ => {
            let mut p = contract::WindowPayload::new(contract::RecordKind::Foreground, at);
            p.app_name = Some("ブラウザ".into());
            p.exe_path = Some(r"C:\apps\b.exe".into());
            p.process_name = Some("b.exe".into());
            p.title = Some("題名".into());
            p.url = Some("https://example.com/a?q=1#f".into());
            let zone = Zone {
                id: "Asia/Tokyo".into(),
                offset_min: 540,
            };
            serde_json::to_string(&[contract::IngestRequest::of(&p, user, "pc-01", at, &zone)?])?
        }
    };
    std::io::stdout().write_all(body.as_bytes())?;
    Ok(())
}
