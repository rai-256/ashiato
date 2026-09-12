// SPDX-License-Identifier: AGPL-3.0-only
//! 未送信をまとめて送る（`docs/collector-contract.md` §送る形）。
//!
//! **`accepted` だけを見て取り除く**（tasks 8.2）。`duplicate` は行が増えていない
//! ことしか言っておらず、断られた理由は `error` にある。
//!
//! **一部の失敗で全部やり直さない** —— 1 件の恒久的な失敗が後続を永久に止めるため。
//! 取り込み口は冪等なので、成功したものを再送しても行は増えない（FR-22）。
use std::collections::HashSet;

use crate::contract::Outboxable;
use crate::outbox::Outbox;
use crate::telemetry;

/// 1 回に載せる件数（C-01 と同じ）。切らないと、長い停止のあと 1 回の POST が
/// 読み取り上限を超え、**1 件も取り除けないまま永久に繰り返す**。
pub const MAX_BATCH: usize = 200;

/// 取り込み口の応答。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    /// 状態符号。200 = 1 件以上を受け付けた / 400 = 1 件も受け付けなかった
    pub status: u16,
    pub body: String,
}

/// 1 回の POST。**到達できなければ `Err`** —— 網の失敗と、サーバが返した応答は別物。
pub trait Transport: std::fmt::Debug {
    /// `path` は `/ingest` か `/heartbeat`。
    fn post(&self, path: &str, body: &str) -> anyhow::Result<Reply>;
}

/// 1 件ごとの結果（送った順に並ぶ。**位置で対応づける**）。
///
/// `id` は断られたとき null のことがあるので、**対応づけに使わない**。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ItemResult {
    /// 受理された（＝未送信から取り除いてよい）
    pub accepted: bool,
    /// 断った理由の種別
    #[serde(default)]
    pub error: Option<String>,
}

/// 送った件数と受け付けられた件数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Flushed {
    /// 送った件数
    pub sent: usize,
    /// 受理された件数
    pub accepted: usize,
}

/// 未送信を送る係。**断られた分を先頭に居座らせない**ために覚えておく。
#[derive(Debug)]
pub struct Sender {
    path: &'static str,
    skipped: HashSet<uuid::Uuid>,
}

impl Sender {
    /// 記録を送る係（`/ingest`）。
    pub fn ingest() -> Self {
        Self {
            path: "/ingest",
            skipped: HashSet::new(),
        }
    }

    /// 生存信号を送る係（`/heartbeat`）。
    pub fn heartbeat() -> Self {
        Self {
            path: "/heartbeat",
            skipped: HashSet::new(),
        }
    }

    /// 1 回送る。**受理された分だけを未送信から取り除く。**
    pub fn flush<T: Outboxable>(
        &mut self,
        outbox: &mut Outbox<T>,
        transport: &dyn Transport,
        log: &mut dyn FnMut(String),
    ) -> anyhow::Result<Flushed> {
        let pending: Vec<T> = outbox.snapshot().to_vec();
        let fresh: Vec<&T> = pending
            .iter()
            .filter(|i| !self.skipped.contains(&i.id()))
            .collect();
        // 全部が「断られた分」なら、もう一度だけ当たり直す（サーバ側の一時的な事情かもしれない）
        let retry_all = fresh.is_empty() && !pending.is_empty();
        if retry_all {
            self.skipped.clear();
        }
        let batch: Vec<&T> = if retry_all {
            pending.iter().take(MAX_BATCH).collect()
        } else {
            fresh.into_iter().take(MAX_BATCH).collect()
        };
        if batch.is_empty() {
            return Ok(Flushed {
                sent: 0,
                accepted: 0,
            });
        }

        let body = serde_json::to_string(&batch)?;
        let reply = match transport.post(self.path, &body) {
            Ok(r) => r,
            Err(e) => {
                // **一時的な失敗。未送信はそのまま残す**（FR-10）
                log(telemetry::line(
                    "send_failed",
                    Some(batch.len()),
                    None,
                    Some(telemetry::error_kind(&e)),
                ));
                return Ok(Flushed {
                    sent: batch.len(),
                    accepted: 0,
                });
            }
        };

        let results: Vec<ItemResult> = match serde_json::from_str(&reply.body) {
            Ok(r) => r,
            Err(_) => {
                // 契約から外れた応答。**何も取り除かない**（取り除くと消える）
                log(telemetry::line(
                    "send_reply_unreadable",
                    Some(batch.len()),
                    None,
                    Some(&reply.status.to_string()),
                ));
                return Ok(Flushed {
                    sent: batch.len(),
                    accepted: 0,
                });
            }
        };

        let mut remove = Vec::new();
        for (item, res) in batch.iter().zip(results.iter()) {
            if res.accepted {
                remove.push(item.id());
            } else {
                // **捨てない。** 生存信号は「動いていた」の証拠で、記録は取り直せない。
                // 先頭に居座らせないために飛ばすだけにする（ST02 の review R18 / H-1）
                self.skipped.insert(item.id());
                log(telemetry::line(
                    "send_rejected",
                    Some(1),
                    None,
                    res.error.as_deref(),
                ));
            }
        }
        let accepted = remove.len();
        outbox.remove(&remove)?;
        log(telemetry::line("sent", Some(accepted), None, None));
        Ok(Flushed {
            sent: batch.len(),
            accepted,
        })
    }
}

/// 実際に HTTP を叩く係。**TLS を持たない**（design D16。オンプレ前提）。
#[derive(Debug)]
pub struct HttpTransport {
    base_url: String,
    token: String,
}

impl HttpTransport {
    /// 接続先と合言葉。**合言葉はログに出さない**（`Debug` にも出ないよう
    /// `telemetry` 以外へ渡さない）。
    pub fn new(base_url: &str, token: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            token: token.to_string(),
        }
    }
}

impl Transport for HttpTransport {
    fn post(&self, path: &str, body: &str) -> anyhow::Result<Reply> {
        let url = format!("{}{path}", self.base_url);
        let res = ureq::post(&url)
            .header("authorization", &format!("Bearer {}", self.token))
            .header("content-type", "application/json")
            .send(body);
        // **400 は応答であって網の失敗ではない**（1 件ごとの結果が本文にある）
        let mut res = match res {
            Ok(r) => r,
            Err(ureq::Error::StatusCode(code)) => {
                return Ok(Reply {
                    status: code,
                    body: String::new(),
                })
            }
            Err(e) => return Err(anyhow::anyhow!("post failed: {}", e)),
        };
        let status = res.status().as_u16();
        let body = res.body_mut().read_to_string()?;
        Ok(Reply { status, body })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::contract::{IngestRequest, RecordKind, WindowPayload};
    use std::cell::RefCell;

    fn tmp_path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "ashiato-send-{}/outbox.jsonl",
            uuid::Uuid::new_v4()
        ))
    }

    fn req(n: u32) -> IngestRequest {
        let at = chrono::DateTime::parse_from_rfc3339("2026-09-13T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc)
            + chrono::Duration::seconds(n as i64);
        let mut p = WindowPayload::new(RecordKind::Foreground, at);
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

    /// 応答を決め打ちする偽の取り込み口。**受け取った本文も覚える。**
    #[derive(Debug, Default)]
    struct FakeTransport {
        /// `None` = 到達できない
        replies: RefCell<Vec<Option<Reply>>>,
        seen: RefCell<Vec<String>>,
    }

    impl FakeTransport {
        fn with(replies: Vec<Option<Reply>>) -> Self {
            Self {
                replies: RefCell::new(replies),
                seen: RefCell::new(Vec::new()),
            }
        }
    }

    impl Transport for FakeTransport {
        fn post(&self, _path: &str, body: &str) -> anyhow::Result<Reply> {
            self.seen.borrow_mut().push(body.to_string());
            let mut r = self.replies.borrow_mut();
            let next = if r.is_empty() { None } else { r.remove(0) };
            match next {
                Some(reply) => Ok(reply),
                None => Err(anyhow::anyhow!("到達できない")),
            }
        }
    }

    fn ok_reply(accepted: &[bool]) -> Option<Reply> {
        let items: Vec<serde_json::Value> = accepted
            .iter()
            .map(|a| {
                serde_json::json!({"id": null, "duplicate": false, "accepted": a,
                                   "error": if *a { serde_json::Value::Null }
                                            else { serde_json::json!("unknown_source") }})
            })
            .collect();
        Some(Reply {
            status: 200,
            body: serde_json::to_string(&items).unwrap(),
        })
    }

    /// **受理された分だけを取り除く**（tasks 8.2）。
    #[test]
    fn outbox_uses_accepted_only() {
        let mut o: Outbox<IngestRequest> = Outbox::open(tmp_path()).unwrap();
        o.add(req(1)).unwrap();
        o.add(req(2)).unwrap();
        let t = FakeTransport::with(vec![ok_reply(&[true, false])]);
        let mut s = Sender::ingest();
        let mut log = |_: String| {};
        let f = s.flush(&mut o, &t, &mut log).unwrap();
        assert_eq!(
            f,
            Flushed {
                sent: 2,
                accepted: 1
            }
        );
        assert_eq!(o.len(), 1, "断られた 1 件が捨てられている");
        assert_eq!(o.snapshot()[0].payload["app_name"], "app-2");
    }

    /// **到達できない間の記録が後から届く**（FR-10 / design D3）。
    ///
    /// Scenario: 到達できない間の記録が後から届く
    #[test]
    fn outbox_survives_outage() {
        let path = tmp_path();
        let mut o: Outbox<IngestRequest> = Outbox::open(path.clone()).unwrap();
        let mut s = Sender::ingest();
        let mut log = |_: String| {};

        // 取り込み口が止まっている間に 2 件生まれる
        o.add(req(1)).unwrap();
        o.add(req(2)).unwrap();
        let down = FakeTransport::with(vec![None]);
        s.flush(&mut o, &down, &mut log).unwrap();
        assert_eq!(o.len(), 2, "到達できないことを理由に捨てている");

        // 起動をまたいでも残る（プロセスが立て直されても消えない）
        drop(o);
        let mut o: Outbox<IngestRequest> = Outbox::open(path).unwrap();
        assert_eq!(o.len(), 2);

        // 戻ったら送られる
        let up = FakeTransport::with(vec![ok_reply(&[true, true])]);
        let f = s.flush(&mut o, &up, &mut log).unwrap();
        assert_eq!(f.accepted, 2);
        assert!(o.is_empty());
        let sent: Vec<serde_json::Value> =
            serde_json::from_str(&up.seen.borrow()[0]).expect("送った本文");
        assert_eq!(sent.len(), 2, "止まっている間の分が送られていない");
    }

    /// 断られた 1 件が**後続を永久に止めない**（ST02 の review R18 / H-1 と同型）。
    #[test]
    fn rejected_item_does_not_block_the_rest() {
        let mut o: Outbox<IngestRequest> = Outbox::open(tmp_path()).unwrap();
        o.add(req(1)).unwrap();
        let t = FakeTransport::with(vec![ok_reply(&[false]), ok_reply(&[true])]);
        let mut s = Sender::ingest();
        let mut log = |_: String| {};
        s.flush(&mut o, &t, &mut log).unwrap();
        assert_eq!(o.len(), 1);

        // 新しい 1 件が積まれたら、そちらが先に送られる
        o.add(req(2)).unwrap();
        s.flush(&mut o, &t, &mut log).unwrap();
        let second: Vec<serde_json::Value> =
            serde_json::from_str(&t.seen.borrow()[1]).expect("2 回目の本文");
        assert_eq!(second.len(), 1);
        assert_eq!(second[0]["payload"]["app_name"], "app-2");
    }

    /// 契約から外れた応答では**何も取り除かない**（取り除くと消える）。
    #[test]
    fn unreadable_reply_removes_nothing() {
        let mut o: Outbox<IngestRequest> = Outbox::open(tmp_path()).unwrap();
        o.add(req(1)).unwrap();
        let t = FakeTransport::with(vec![Some(Reply {
            status: 500,
            body: "<html>".into(),
        })]);
        let mut s = Sender::ingest();
        let mut log = |_: String| {};
        let f = s.flush(&mut o, &t, &mut log).unwrap();
        assert_eq!(f.accepted, 0);
        assert_eq!(o.len(), 1);
    }
}
