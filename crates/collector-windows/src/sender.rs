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

        let results: Vec<ItemResult> = match serde_json::from_str::<Vec<ItemResult>>(&reply.body) {
            // **件数が合わない応答は読まない**（R21）。位置で対応づけるので、
            // ずれたまま読むと断られた 1 件を「受理」として取り除いてしまう
            Ok(r) if r.len() == batch.len() => r,
            _ => {
                // 契約から外れた応答。**何も取り除かない**（取り除くと消える）
                let kind = match reply.status {
                    401 | 403 => "send_unauthorized",
                    _ => "send_reply_unreadable",
                };
                log(telemetry::line(
                    kind,
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
        // 取り除いた分は覚えておく必要が無い（覚えたままにすると単調に増える。R30）
        let still: std::collections::HashSet<uuid::Uuid> =
            outbox.snapshot().iter().map(|i| i.id()).collect();
        self.skipped.retain(|id| still.contains(id));
        log(telemetry::line("sent", Some(accepted), None, None));
        Ok(Flushed {
            sent: batch.len(),
            accepted,
        })
    }
}

/// 接続を張るまでの上限（R22）。
pub const CONNECT_TIMEOUT_SEC: u64 = 10;
/// 1 回の要求全体の上限（R22）。**見回りの輪の中で呼ぶので、無期限に待たない** ——
/// 待っている間は前景を 1 度も観測できず、2 分を超えると眠っていたことにされる。
pub const REQUEST_TIMEOUT_SEC: u64 = 30;

/// 状態符号を「失敗」にしない `ureq` の係。**400 の本文を読むため**（R22）——
/// 既定のままだと 400 で本文ごと捨てられ、1 件ごとの理由が読めず、
/// 断られた同じ 200 件を永久に送り直す。
pub fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_connect(Some(std::time::Duration::from_secs(CONNECT_TIMEOUT_SEC)))
        .timeout_global(Some(std::time::Duration::from_secs(REQUEST_TIMEOUT_SEC)))
        .build()
        .new_agent()
}

/// 実際に HTTP を叩く係。**TLS を持たない**（design D16。オンプレ前提）。
pub struct HttpTransport {
    base_url: String,
    token: String,
    agent: ureq::Agent,
}

/// **合言葉を出さない**（R11）。`{:?}` 1 つで PERM-8 の合言葉がログに落ちる。
impl std::fmt::Debug for HttpTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpTransport")
            .field("base_url", &self.base_url)
            .field("token", &"***")
            .finish()
    }
}

impl HttpTransport {
    /// 接続先と合言葉。
    pub fn new(base_url: &str, token: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            token: token.to_string(),
            agent: agent(),
        }
    }
}

impl Transport for HttpTransport {
    fn post(&self, path: &str, body: &str) -> anyhow::Result<Reply> {
        let url = format!("{}{path}", self.base_url);
        let mut res = self
            .agent
            .post(&url)
            .header("authorization", &format!("Bearer {}", self.token))
            .header("content-type", "application/json")
            .send(body)
            // 網の失敗。**例外の文言は外へ出さない**（本文が混ざることがある）
            .map_err(|e| anyhow::anyhow!("post failed: {}", e))?;
        let status = res.status().as_u16();
        // **400 も本文を読む**（1 件ごとの結果がそこにある）
        let body = res.body_mut().read_to_string().unwrap_or_default();
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

    /// **件数の合わない応答では何も取り除かない**（R21）。
    #[test]
    fn mismatched_reply_length_removes_nothing() {
        let mut o: Outbox<IngestRequest> = Outbox::open(tmp_path()).unwrap();
        o.add(req(1)).unwrap();
        o.add(req(2)).unwrap();
        let t = FakeTransport::with(vec![ok_reply(&[true])]);
        let mut s = Sender::ingest();
        let mut log = |_: String| {};
        let f = s.flush(&mut o, &t, &mut log).unwrap();
        assert_eq!(f.accepted, 0);
        assert_eq!(o.len(), 2, "ずれた応答で取り除いた");
    }

    /// 断られた分**だけ**が残ったら、次の契機で当たり直す（I5）。
    #[test]
    fn rejected_only_backlog_is_retried() {
        let mut o: Outbox<IngestRequest> = Outbox::open(tmp_path()).unwrap();
        o.add(req(1)).unwrap();
        let t = FakeTransport::with(vec![ok_reply(&[false]), ok_reply(&[true])]);
        let mut s = Sender::ingest();
        let mut log = |_: String| {};
        s.flush(&mut o, &t, &mut log).unwrap();
        let f = s.flush(&mut o, &t, &mut log).unwrap();
        assert_eq!(f.sent, 1, "断られた分しか無いのに当たり直さない");
        assert!(o.is_empty());
    }

    /// **1 回に載せる件数を切る**（I9）。
    #[test]
    fn batch_is_capped() {
        let mut o: Outbox<IngestRequest> = Outbox::open(tmp_path()).unwrap();
        for n in 0..(MAX_BATCH as u32 + 50) {
            o.add(req(n)).unwrap();
        }
        let accepted = vec![true; MAX_BATCH];
        let t = FakeTransport::with(vec![ok_reply(&accepted)]);
        let mut s = Sender::ingest();
        let mut log = |_: String| {};
        let f = s.flush(&mut o, &t, &mut log).unwrap();
        assert_eq!(f.sent, MAX_BATCH);
        assert_eq!(o.len(), 50);
    }

    /// 1 回だけ応答する本物の HTTP の相手。受け取った要求の頭と本文を返す。
    fn one_shot_server(
        status: &'static str,
        body: String,
    ) -> (String, std::thread::JoinHandle<String>) {
        use std::io::{Read as _, Write as _};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = format!("http://{}", listener.local_addr().unwrap());
        let h = std::thread::spawn(move || {
            let (mut sock, _) = listener.accept().unwrap();
            let mut buf = Vec::new();
            let mut chunk = [0u8; 4096];
            // 頭と Content-Length ぶんの本文を読む
            loop {
                let n = sock.read(&mut chunk).unwrap();
                buf.extend_from_slice(&chunk[..n]);
                let text = String::from_utf8_lossy(&buf).to_string();
                if let Some(end) = text.find("\r\n\r\n") {
                    let len = text
                        .lines()
                        .find_map(|l| {
                            l.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|v| v.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if buf.len() >= end + 4 + len || n == 0 {
                        break;
                    }
                }
            }
            let reply = format!(
                "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            sock.write_all(reply.as_bytes()).unwrap();
            String::from_utf8_lossy(&buf).to_string()
        });
        (addr, h)
    }

    /// **本物の HTTP で合言葉を送り、400 の本文を読む**（R5 / R22）。
    /// 400 は「1 件も受け付けなかった」で、理由は本文の 1 件ごとの結果にある。
    #[test]
    fn http_transport_sends_bearer_and_reads_400_body() {
        let body = serde_json::json!([{"id": null, "duplicate": false,
                                        "accepted": false, "error": "unknown_source"}])
        .to_string();
        let (base, server) = one_shot_server("400 Bad Request", body);
        let t = HttpTransport::new(&base, "secret-token-0123456789");
        let mut o: Outbox<IngestRequest> = Outbox::open(tmp_path()).unwrap();
        o.add(req(1)).unwrap();
        let mut s = Sender::ingest();
        let mut lines = Vec::new();
        let mut log = |l: String| lines.push(l);
        let f = s.flush(&mut o, &t, &mut log).unwrap();
        let seen = server.join().unwrap();

        assert!(
            seen.to_ascii_lowercase()
                .contains("authorization: bearer secret-token-0123456789"),
            "合言葉が送られていない: {seen}"
        );
        assert!(seen.starts_with("POST /ingest "), "{seen}");
        assert_eq!(f.accepted, 0);
        assert_eq!(o.len(), 1, "断られた記録を捨てた");
        assert!(
            lines.iter().any(|l| l.contains("error=unknown_source")),
            "400 の本文の理由が読めていない: {lines:?}"
        );
        // 合言葉は Debug にも出ない（R11）
        assert!(!format!("{t:?}").contains("secret-token"));
    }

    /// 応答しない相手を**無期限に待たない**（R22）。
    #[test]
    fn http_transport_has_timeouts() {
        const { assert!(REQUEST_TIMEOUT_SEC < crate::runtime::SUSPEND_GAP_SEC as u64) };
        const { assert!(CONNECT_TIMEOUT_SEC <= REQUEST_TIMEOUT_SEC) };
    }
}
