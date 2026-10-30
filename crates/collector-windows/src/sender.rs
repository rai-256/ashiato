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
    /// `path` は `/ingest` か `/heartbeat`。`contract_version` は要求の見出し
    /// `Ashiato-Contract-Version` に載せる、その回に載せた行の版の最大（ST50 / design D9）。
    fn post(&self, path: &str, body: &str, contract_version: u32) -> anyhow::Result<Reply>;

    /// `path` は `/contract`（S-01 が受け付ける版を訊く。design D18（仮））。
    fn get(&self, path: &str) -> anyhow::Result<Reply>;
}

/// `GET /contract` の答え（design D11）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractAnswer {
    /// S-01 が受け付ける版 A
    Version(u32),
    /// 口が無い（404）。受け付けないものとして扱い、すぐ更新を促す
    NoEndpoint,
    /// 200 だが読めない（本文が JSON でない・版が無い・正の整数でない）。送らずに残し、続いたら促す（D22）
    Unreadable,
    /// 到達できない・時間切れ・401・403・5xx・そのほか。更新が原因ではないので止まった状態にしない
    Transient,
}

/// 確かめの返事を「版 A / 受け付けない / 一時的な失敗」に読む（design D11）。
pub fn read_contract_answer(reply: &anyhow::Result<Reply>) -> ContractAnswer {
    let Ok(reply) = reply else {
        return ContractAnswer::Transient;
    };
    match reply.status {
        404 => ContractAnswer::NoEndpoint,
        200 => serde_json::from_str::<serde_json::Value>(&reply.body)
            .ok()
            .and_then(|v| v.get("contract_version")?.as_u64())
            .filter(|n| *n >= 1)
            .and_then(|n| u32::try_from(n).ok())
            .map_or(ContractAnswer::Unreadable, ContractAnswer::Version),
        _ => ContractAnswer::Transient,
    }
}

/// 直前の `flush` が見た版の事情（design D9 の（a）（c）（d））。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SendOutcome {
    /// 1 回送った（要求を出した）
    pub posted: bool,
    /// 版 > A の行が未送信にある（a）
    pub ahead: bool,
    /// 426 で断られた（c）
    pub refused: bool,
    /// 1 件以上が `unknown_field` か `contract_unsupported` だった（d）
    pub rejected_by_version: bool,
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
    last: SendOutcome,
}

impl Sender {
    /// 記録を送る係（`/ingest`）。
    pub fn ingest() -> Self {
        Self {
            path: "/ingest",
            skipped: HashSet::new(),
            last: SendOutcome::default(),
        }
    }

    /// 生存信号を送る係（`/heartbeat`）。
    pub fn heartbeat() -> Self {
        Self {
            path: "/heartbeat",
            skipped: HashSet::new(),
            last: SendOutcome::default(),
        }
    }

    /// 直前の `flush` が見た版の事情。
    pub fn last_outcome(&self) -> SendOutcome {
        self.last
    }

    /// 「飛ばす」印を捨てる。止まった状態を抜けた契機で呼ぶ（design D14）。
    pub fn clear_skipped(&mut self) {
        self.skipped.clear();
    }

    /// 1 回送る。**受理された分だけを未送信から取り除く。**
    ///
    /// `accepted_version` は S-01 が受け付ける版 A。**版 ≤ A の行だけ**を送り、見出しには載せた行の最大を入れる。
    /// 版 > A の行は捨ても飛ばしもしない（design D9）。`None`（答えが読めない契機）は何も送らない。
    pub fn flush<T: Outboxable>(
        &mut self,
        outbox: &mut Outbox<T>,
        transport: &dyn Transport,
        accepted_version: Option<u32>,
        log: &mut dyn FnMut(String),
    ) -> anyhow::Result<Flushed> {
        self.last = SendOutcome::default();
        let pending: Vec<T> = outbox.snapshot().to_vec();
        let Some(a) = accepted_version else {
            return Ok(Flushed {
                sent: 0,
                accepted: 0,
            });
        };
        self.last.ahead = pending.iter().any(|i| i.contract_version() > a);
        let eligible: Vec<&T> = pending
            .iter()
            .filter(|i| i.contract_version() <= a)
            .collect();
        let fresh: Vec<&T> = eligible
            .iter()
            .copied()
            .filter(|i| !self.skipped.contains(&i.id()))
            .collect();
        // 全部が「断られた分」なら、もう一度だけ当たり直す（サーバ側の一時的な事情かもしれない）
        let retry_all = fresh.is_empty() && !eligible.is_empty();
        if retry_all {
            self.skipped.clear();
        }
        let mut candidates: Vec<&T> = if retry_all { eligible } else { fresh };
        // 後回しにするもの（履歴）は、そうでないものの後ろへ。どちらの中でも積んだ順のまま（R69）
        candidates.sort_by_key(|i| i.sends_late());
        let batch: Vec<&T> = candidates.into_iter().take(MAX_BATCH).collect();
        if batch.is_empty() {
            return Ok(Flushed {
                sent: 0,
                accepted: 0,
            });
        }
        let header_version = batch
            .iter()
            .map(|i| i.contract_version())
            .max()
            .unwrap_or(a);

        let body = serde_json::to_string(&batch)?;
        self.last.posted = true;
        let reply = match transport.post(self.path, &body, header_version) {
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

        // **426 は何も取り除かない**（捨てない側。飛ばす印も付けない。design D4・D17）
        if reply.status == 426 {
            self.last.refused = true;
            log(telemetry::line(
                "send_contract_unsupported",
                Some(batch.len()),
                None,
                Some("426"),
            ));
            return Ok(Flushed {
                sent: batch.len(),
                accepted: 0,
            });
        }

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
                if matches!(
                    res.error.as_deref(),
                    Some("unknown_field" | "contract_unsupported")
                ) {
                    self.last.rejected_by_version = true;
                }
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
    fn post(&self, path: &str, body: &str, contract_version: u32) -> anyhow::Result<Reply> {
        let url = format!("{}{path}", self.base_url);
        let mut res = self
            .agent
            .post(&url)
            .header("authorization", &format!("Bearer {}", self.token))
            .header("ashiato-contract-version", &contract_version.to_string())
            .header("content-type", "application/json")
            .send(body)
            // 網の失敗。**例外の文言は外へ出さない**（本文が混ざることがある）
            .map_err(|e| anyhow::anyhow!("post failed: {}", e))?;
        let status = res.status().as_u16();
        // **400 も本文を読む**（1 件ごとの結果がそこにある）
        let body = res.body_mut().read_to_string().unwrap_or_default();
        Ok(Reply { status, body })
    }

    fn get(&self, path: &str) -> anyhow::Result<Reply> {
        let url = format!("{}{path}", self.base_url);
        let mut res = self
            .agent
            .get(&url)
            .header("authorization", &format!("Bearer {}", self.token))
            .call()
            .map_err(|e| anyhow::anyhow!("get failed: {}", e))?;
        let status = res.status().as_u16();
        let body = res.body_mut().read_to_string().unwrap_or_default();
        Ok(Reply { status, body })
    }
}

/// 途切れの問い合わせの打ち切り。**通知の遅れは収集を止めない**ので短い（design D11）。
pub const OUTAGE_QUERY_TIMEOUT_SEC: u64 = 10;

/// `GET /outages?device_id=&user_id=`（ST14 / design D6・D29（仮））。届かない・読めないは `Unreachable`。
/// 利用者を付けないとサーバは既定の利用者で判定する（review/code.md R13）。
pub struct HttpOutageQuery {
    base_url: String,
    token: String,
    user_id: uuid::Uuid,
    agent: ureq::Agent,
}

/// **合言葉を出さない**（R11）。
impl std::fmt::Debug for HttpOutageQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpOutageQuery")
            .field("base_url", &self.base_url)
            .field("token", &"***")
            .field("user_id", &self.user_id)
            .finish()
    }
}

impl HttpOutageQuery {
    pub fn new(base_url: &str, token: &str, user_id: uuid::Uuid) -> Self {
        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(std::time::Duration::from_secs(
                OUTAGE_QUERY_TIMEOUT_SEC,
            )))
            .build()
            .new_agent();
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            token: token.to_string(),
            user_id,
            agent,
        }
    }
}

impl crate::outage_notice::OutageQuery for HttpOutageQuery {
    fn fetch(&self, device_id: &str) -> crate::outage_notice::OutageResult {
        let sent = self
            .agent
            .get(&format!("{}/outages", self.base_url))
            .query("device_id", device_id)
            .query("user_id", self.user_id.to_string())
            .header("authorization", &format!("Bearer {}", self.token))
            .call();
        let Ok(mut res) = sent else {
            return crate::outage_notice::OutageResult::Unreachable;
        };
        let status = res.status().as_u16();
        let body = res.body_mut().read_to_string().unwrap_or_default();
        crate::outage_notice::parse_reply(&Reply { status, body })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::contract::{IngestRequest, RecordKind, WindowPayload};
    use std::cell::RefCell;

    /// 試験の最後まで `TempDir` を持つ（落ちても消える）。
    fn tmp_path() -> (tempfile::TempDir, std::path::PathBuf) {
        let tmp = tempfile::Builder::new()
            .prefix("ashiato-send-")
            .tempdir()
            .unwrap();
        let path = tmp.path().join("outbox.jsonl");
        (tmp, path)
    }

    thread_local! {
        /// 試験の置き場。試験の最後（スレッドの終わり）まで持ち、落ちても消える
        static DIRS: RefCell<Vec<tempfile::TempDir>> = const { RefCell::new(Vec::new()) };
    }

    fn kept_tmp_path() -> std::path::PathBuf {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("outbox.jsonl");
        DIRS.with(|d| d.borrow_mut().push(dir));
        path
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
        /// 見出しに載せた版
        versions: RefCell<Vec<u32>>,
        /// `GET /contract` の返事（`None` = 到達できない）。空なら 404
        contract: RefCell<Vec<Option<Reply>>>,
    }

    impl FakeTransport {
        fn with(replies: Vec<Option<Reply>>) -> Self {
            Self {
                replies: RefCell::new(replies),
                seen: RefCell::new(Vec::new()),
                versions: RefCell::new(Vec::new()),
                contract: RefCell::new(Vec::new()),
            }
        }
    }

    impl Transport for FakeTransport {
        fn post(&self, _path: &str, body: &str, version: u32) -> anyhow::Result<Reply> {
            self.seen.borrow_mut().push(body.to_string());
            self.versions.borrow_mut().push(version);
            let mut r = self.replies.borrow_mut();
            let next = if r.is_empty() { None } else { r.remove(0) };
            match next {
                Some(reply) => Ok(reply),
                None => Err(anyhow::anyhow!("到達できない")),
            }
        }

        fn get(&self, _path: &str) -> anyhow::Result<Reply> {
            let mut r = self.contract.borrow_mut();
            if r.is_empty() {
                return Ok(Reply {
                    status: 404,
                    body: String::new(),
                });
            }
            match r.remove(0) {
                Some(reply) => Ok(reply),
                None => Err(anyhow::anyhow!("到達できない")),
            }
        }
    }

    fn version_reply(v: u32) -> Option<Reply> {
        Some(Reply {
            status: 200,
            body: format!(r#"{{"contract_version":{v}}}"#),
        })
    }

    fn at_version(n: u32, v: u32) -> IngestRequest {
        let mut r = req(n);
        r.contract_version = v;
        r
    }

    fn app_names(body: &str) -> Vec<String> {
        let sent: Vec<serde_json::Value> = serde_json::from_str(body).unwrap();
        sent.iter()
            .map(|i| i["payload"]["app_name"].as_str().unwrap().to_string())
            .collect()
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
        let (_tmp, path) = tmp_path();
        let mut o: Outbox<IngestRequest> = Outbox::open(path).unwrap();
        o.add(req(1)).unwrap();
        o.add(req(2)).unwrap();
        let t = FakeTransport::with(vec![ok_reply(&[true, false])]);
        let mut s = Sender::ingest();
        let mut log = |_: String| {};
        let f = s.flush(&mut o, &t, Some(1), &mut log).unwrap();
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
        let (_tmp, path) = tmp_path();
        let mut o: Outbox<IngestRequest> = Outbox::open(path.clone()).unwrap();
        let mut s = Sender::ingest();
        let mut log = |_: String| {};

        // 取り込み口が止まっている間に 2 件生まれる
        o.add(req(1)).unwrap();
        o.add(req(2)).unwrap();
        let down = FakeTransport::with(vec![None]);
        s.flush(&mut o, &down, Some(1), &mut log).unwrap();
        assert_eq!(o.len(), 2, "到達できないことを理由に捨てている");

        // 起動をまたいでも残る（プロセスが立て直されても消えない）
        drop(o);
        let mut o: Outbox<IngestRequest> = Outbox::open(path).unwrap();
        assert_eq!(o.len(), 2);

        // 戻ったら送られる
        let up = FakeTransport::with(vec![ok_reply(&[true, true])]);
        let f = s.flush(&mut o, &up, Some(1), &mut log).unwrap();
        assert_eq!(f.accepted, 2);
        assert!(o.is_empty());
        let sent: Vec<serde_json::Value> =
            serde_json::from_str(&up.seen.borrow()[0]).expect("送った本文");
        assert_eq!(sent.len(), 2, "止まっている間の分が送られていない");
    }

    /// 断られた 1 件が**後続を永久に止めない**（ST02 の review R18 / H-1 と同型）。
    #[test]
    fn rejected_item_does_not_block_the_rest() {
        let (_tmp, path) = tmp_path();
        let mut o: Outbox<IngestRequest> = Outbox::open(path).unwrap();
        o.add(req(1)).unwrap();
        let t = FakeTransport::with(vec![ok_reply(&[false]), ok_reply(&[true])]);
        let mut s = Sender::ingest();
        let mut log = |_: String| {};
        s.flush(&mut o, &t, Some(1), &mut log).unwrap();
        assert_eq!(o.len(), 1);

        // 新しい 1 件が積まれたら、そちらが先に送られる
        o.add(req(2)).unwrap();
        s.flush(&mut o, &t, Some(1), &mut log).unwrap();
        let second: Vec<serde_json::Value> =
            serde_json::from_str(&t.seen.borrow()[1]).expect("2 回目の本文");
        assert_eq!(second.len(), 1);
        assert_eq!(second[0]["payload"]["app_name"], "app-2");
    }

    /// 契約から外れた応答では**何も取り除かない**（取り除くと消える）。
    #[test]
    fn unreadable_reply_removes_nothing() {
        let (_tmp, path) = tmp_path();
        let mut o: Outbox<IngestRequest> = Outbox::open(path).unwrap();
        o.add(req(1)).unwrap();
        let t = FakeTransport::with(vec![Some(Reply {
            status: 500,
            body: "<html>".into(),
        })]);
        let mut s = Sender::ingest();
        let mut log = |_: String| {};
        let f = s.flush(&mut o, &t, Some(1), &mut log).unwrap();
        assert_eq!(f.accepted, 0);
        assert_eq!(o.len(), 1);
    }

    /// **件数の合わない応答では何も取り除かない**（R21）。
    #[test]
    fn mismatched_reply_length_removes_nothing() {
        let (_tmp, path) = tmp_path();
        let mut o: Outbox<IngestRequest> = Outbox::open(path).unwrap();
        o.add(req(1)).unwrap();
        o.add(req(2)).unwrap();
        let t = FakeTransport::with(vec![ok_reply(&[true])]);
        let mut s = Sender::ingest();
        let mut log = |_: String| {};
        let f = s.flush(&mut o, &t, Some(1), &mut log).unwrap();
        assert_eq!(f.accepted, 0);
        assert_eq!(o.len(), 2, "ずれた応答で取り除いた");
    }

    /// 断られた分**だけ**が残ったら、次の契機で当たり直す（I5）。
    #[test]
    fn rejected_only_backlog_is_retried() {
        let (_tmp, path) = tmp_path();
        let mut o: Outbox<IngestRequest> = Outbox::open(path).unwrap();
        o.add(req(1)).unwrap();
        let t = FakeTransport::with(vec![ok_reply(&[false]), ok_reply(&[true])]);
        let mut s = Sender::ingest();
        let mut log = |_: String| {};
        s.flush(&mut o, &t, Some(1), &mut log).unwrap();
        let f = s.flush(&mut o, &t, Some(1), &mut log).unwrap();
        assert_eq!(f.sent, 1, "断られた分しか無いのに当たり直さない");
        assert!(o.is_empty());
    }

    /// **1 回に載せる件数を切る**（I9）。
    #[test]
    fn batch_is_capped() {
        let (_tmp, path) = tmp_path();
        let mut o: Outbox<IngestRequest> = Outbox::open(path).unwrap();
        for n in 0..(MAX_BATCH as u32 + 50) {
            o.add(req(n)).unwrap();
        }
        let accepted = vec![true; MAX_BATCH];
        let t = FakeTransport::with(vec![ok_reply(&accepted)]);
        let mut s = Sender::ingest();
        let mut log = |_: String| {};
        let f = s.flush(&mut o, &t, Some(1), &mut log).unwrap();
        assert_eq!(f.sent, MAX_BATCH);
        assert_eq!(o.len(), 50);
    }

    /// 履歴の大きな取り込みの後ろに積んだウィンドウの記録が、次の 1 回で出る（R69。design D3（仮））。
    #[test]
    fn history_backlog_does_not_hold_back_window_records() {
        let (_tmp, path) = tmp_path();
        let mut o: Outbox<IngestRequest> = Outbox::open(path).unwrap();
        for n in 0..(MAX_BATCH as u32 * 3) {
            let mut r = req(n);
            r.logical_source = crate::history::contract::LOGICAL_SOURCE.to_string();
            o.add(r).unwrap();
        }
        let window = req(9999);
        o.add(window.clone()).unwrap();
        let t = FakeTransport::with(vec![ok_reply(&[true; MAX_BATCH])]);
        let mut s = Sender::ingest();
        let mut log = |_: String| {};
        let f = s.flush(&mut o, &t, Some(1), &mut log).unwrap();
        assert_eq!(f.sent, MAX_BATCH);
        assert!(
            !o.snapshot().iter().any(|i| i.id == window.id),
            "ウィンドウの記録が履歴の後ろで待っている"
        );
        // 履歴どうしは積んだ順のまま（先頭の MAX_BATCH - 1 件が出て、残りは後ろから）
        assert_eq!(o.len(), MAX_BATCH * 2 + 1);
        assert_eq!(
            o.snapshot()[0].event_time,
            req(MAX_BATCH as u32 - 1).event_time
        );
    }

    /// 確かめの返事を読んで、その版で 1 回送る。
    fn flush_after_check(
        s: &mut Sender,
        o: &mut Outbox<IngestRequest>,
        t: &FakeTransport,
    ) -> (ContractAnswer, Flushed) {
        let answer = read_contract_answer(&t.get("/contract"));
        let a = match answer {
            ContractAnswer::Version(a) => Some(a),
            _ => None,
        };
        let mut log = |_: String| {};
        (answer, s.flush(o, t, a, &mut log).unwrap())
    }

    // Scenario: PC は S-01 が受け付ける版の行だけを送り、新しい版の行を残す
    #[test]
    fn st50_sender_sends_accepted_versions_only() {
        let mut o: Outbox<IngestRequest> = Outbox::open(kept_tmp_path()).unwrap();
        o.add_many_at(vec![req(1), req(2)], 1).unwrap();
        o.add_many_at(vec![req(3)], 2).unwrap();
        let t = FakeTransport::with(vec![ok_reply(&[true, true])]);
        t.contract.borrow_mut().push(version_reply(1));
        let mut s = Sender::ingest();
        let (_, f) = flush_after_check(&mut s, &mut o, &t);
        assert_eq!(f.sent, 2);
        assert_eq!(app_names(&t.seen.borrow()[0]), ["app-1", "app-2"]);
        assert_eq!(o.len(), 1, "新しい版の行が残っていない");
        assert_eq!(o.snapshot()[0].contract_version, 2);
        assert!(s.last_outcome().ahead, "版 > A の行があるのに印が立たない");
        assert_eq!(*t.versions.borrow(), [1], "見出しは載せた行の版");
        assert!(s.skipped.is_empty(), "送っていない行に飛ばす印を付けている");
    }

    // Scenario: PC は積んだ時の版を 1 行ごとに持ち、更新の後も変えない
    #[test]
    fn st50_sender_burned_version_kept() {
        let path = kept_tmp_path();
        let mut o: Outbox<IngestRequest> = Outbox::open(path.clone()).unwrap();
        // 版 1 の時に積み、その後に版が上がった（送る側は積んだ版を上書きしない）
        o.add_many_at(vec![req(1)], 1).unwrap();
        o.add_many_at(vec![req(2)], 2).unwrap();
        drop(o);
        let mut o: Outbox<IngestRequest> = Outbox::open(path).unwrap();
        let t = FakeTransport::with(vec![ok_reply(&[true, true])]);
        t.contract.borrow_mut().push(version_reply(2));
        let mut s = Sender::ingest();
        flush_after_check(&mut s, &mut o, &t);
        let sent: Vec<serde_json::Value> = serde_json::from_str(&t.seen.borrow()[0]).unwrap();
        assert_eq!(sent[0]["contract_version"], 1, "更新の後に版が書き換わった");
        assert_eq!(sent[1]["contract_version"], 2);
        // 既定の積み方は今の版を焼く
        let mut o: Outbox<IngestRequest> = Outbox::open(kept_tmp_path()).unwrap();
        o.add(at_version(3, 99)).unwrap();
        assert_eq!(
            o.snapshot()[0].contract_version,
            crate::contract::CONTRACT_VERSION
        );
    }

    // Scenario: PC の版を持たない古い行は版 1 として送られる
    #[test]
    fn st50_sender_unversioned_row_is_v1() {
        let path = kept_tmp_path();
        let mut o: Outbox<IngestRequest> = Outbox::open(path.clone()).unwrap();
        o.add(req(1)).unwrap();
        drop(o);
        // ST50 より前の形の行（版の欄が無い）に書き換える
        let text = std::fs::read_to_string(&path).unwrap();
        let mut v: serde_json::Value = serde_json::from_str(text.trim()).unwrap();
        v.as_object_mut().unwrap().remove("contract_version");
        std::fs::write(&path, format!("{v}\n")).unwrap();
        let mut o: Outbox<IngestRequest> = Outbox::open(path).unwrap();
        assert_eq!(o.snapshot()[0].contract_version, 1);
        let t = FakeTransport::with(vec![ok_reply(&[true])]);
        t.contract.borrow_mut().push(version_reply(1));
        let mut s = Sender::ingest();
        flush_after_check(&mut s, &mut o, &t);
        let sent: Vec<serde_json::Value> = serde_json::from_str(&t.seen.borrow()[0]).unwrap();
        assert_eq!(sent[0]["contract_version"], 1);
        assert!(o.is_empty());
    }

    // Scenario: PC の要求の見出しは載せた行の版の最大になる
    #[test]
    fn st50_sender_header_is_batch_max() {
        let mut o: Outbox<IngestRequest> = Outbox::open(kept_tmp_path()).unwrap();
        o.add_many_at(vec![req(1)], 1).unwrap();
        o.add_many_at(vec![req(2)], 2).unwrap();
        o.add_many_at(vec![req(3)], 3).unwrap();
        // A = 2: 版 1 と 2 が 1 回に混ざり、見出しは 2（A の 2 でも、行の 3 でもない）
        let t = FakeTransport::with(vec![ok_reply(&[true, true])]);
        t.contract.borrow_mut().push(version_reply(2));
        let mut s = Sender::ingest();
        flush_after_check(&mut s, &mut o, &t);
        assert_eq!(*t.versions.borrow(), [2]);
        // A = 5 でも、載せた行の最大（3）。A ではない
        let t = FakeTransport::with(vec![ok_reply(&[true])]);
        t.contract.borrow_mut().push(version_reply(5));
        flush_after_check(&mut s, &mut o, &t);
        assert_eq!(*t.versions.borrow(), [3]);
    }

    // Scenario: 問い合わせの口が無い S-01 には PC も何も送らない
    #[test]
    fn st50_sender_no_contract_endpoint() {
        let readings = [
            Some(Reply {
                status: 404,
                body: String::new(),
            }),
            Some(Reply {
                status: 200,
                body: "<html>sign in</html>".into(),
            }),
            Some(Reply {
                status: 200,
                body: r#"{"other":1}"#.into(),
            }),
            Some(Reply {
                status: 200,
                body: r#"{"contract_version":"2"}"#.into(),
            }),
            Some(Reply {
                status: 200,
                body: r#"{"contract_version":0}"#.into(),
            }),
        ];
        let want = [
            ContractAnswer::NoEndpoint,
            ContractAnswer::Unreadable,
            ContractAnswer::Unreadable,
            ContractAnswer::Unreadable,
            ContractAnswer::Unreadable,
        ];
        for (reading, want) in readings.into_iter().zip(want) {
            let mut o: Outbox<IngestRequest> = Outbox::open(kept_tmp_path()).unwrap();
            o.add(req(1)).unwrap();
            let t = FakeTransport::with(vec![ok_reply(&[true])]);
            t.contract.borrow_mut().push(reading);
            let mut s = Sender::ingest();
            let (answer, f) = flush_after_check(&mut s, &mut o, &t);
            assert_eq!(answer, want);
            assert_eq!(f.sent, 0);
            assert!(t.seen.borrow().is_empty(), "読めない答えなのに送った");
            assert_eq!(o.len(), 1, "行が減っている");
        }
    }

    // Scenario: 426 で断られた PC の要求の行は 1 件も取り除かれない
    #[test]
    fn st50_sender_426_removes_nothing() {
        let mut o: Outbox<IngestRequest> = Outbox::open(kept_tmp_path()).unwrap();
        o.add_many_at(vec![req(1), req(2)], 1).unwrap();
        let refused = Some(Reply {
            status: 426,
            body: r#"{"error":"contract_unsupported","contract_version":1}"#.into(),
        });
        let t = FakeTransport::with(vec![refused, ok_reply(&[true, true])]);
        t.contract
            .borrow_mut()
            .extend([version_reply(1), version_reply(1)]);
        let mut s = Sender::ingest();
        let (_, f) = flush_after_check(&mut s, &mut o, &t);
        assert_eq!(f.accepted, 0);
        assert_eq!(o.len(), 2);
        assert!(s.last_outcome().refused);
        assert!(s.skipped.is_empty(), "426 で飛ばす印を付けた");
        // 印が無いので、次の回も同じ先頭から送られる
        flush_after_check(&mut s, &mut o, &t);
        assert_eq!(app_names(&t.seen.borrow()[1]), ["app-1", "app-2"]);
        assert!(o.is_empty());
    }

    // Scenario: 契約の版で止まっている間も PC は行を捨てない
    #[test]
    fn st50_sender_never_drops_when_held() {
        let mut o: Outbox<IngestRequest> = Outbox::open(kept_tmp_path()).unwrap();
        // 100 日分（1 日 288 行）。半分は新しい版、半分は 404 の間に積んだ版 1
        let rows: Vec<IngestRequest> = (0..100 * 288).map(req).collect();
        let total = rows.len();
        o.add_many_at(rows[..total / 2].to_vec(), 1).unwrap();
        o.add_many_at(rows[total / 2..].to_vec(), 2).unwrap();
        let mut s = Sender::ingest();
        let mut log = |_: String| {};
        for answer in [None, None] {
            s.flush(&mut o, &FakeTransport::default(), answer, &mut log)
                .unwrap();
            assert_eq!(o.len(), total, "答えが読めない間に減った");
        }
        // 版 0 しか受け付けない（どの行も新しい）状態を何度繰り返しても減らない
        for _ in 0..3 {
            let t = FakeTransport::default();
            let f = s.flush(&mut o, &t, Some(0), &mut log).unwrap();
            assert_eq!(f.sent, 0);
            assert!(t.seen.borrow().is_empty());
            assert_eq!(o.len(), total);
        }
        assert!(s.last_outcome().ahead);
    }

    // Scenario: S-01 を更新すると PC に残っていた記録がそのまま入る
    #[test]
    fn st50_sender_resumes_after_update() {
        let mut o: Outbox<IngestRequest> = Outbox::open(kept_tmp_path()).unwrap();
        o.add_many_at(vec![req(1), req(2)], 2).unwrap();
        let rejected = Some(Reply {
            status: 200,
            body: serde_json::json!([
                {"id": null, "duplicate": false, "accepted": false, "error": "unknown_field"},
                {"id": null, "duplicate": false, "accepted": false, "error": "contract_unsupported"}
            ])
            .to_string(),
        });
        let t = FakeTransport::with(vec![rejected, ok_reply(&[true, true])]);
        // 1 回目は古い S-01（A = 2 と答えたが断られた）。2 回目は更新後
        t.contract
            .borrow_mut()
            .extend([version_reply(2), version_reply(2)]);
        let mut s = Sender::ingest();
        flush_after_check(&mut s, &mut o, &t);
        assert!(s.last_outcome().rejected_by_version);
        assert_eq!(o.len(), 2);
        assert_eq!(s.skipped.len(), 2, "飛ばす印が付いていない");
        // 更新した S-01 に戻った契機で印を捨てる（design D14）
        s.clear_skipped();
        assert!(s.skipped.is_empty());
        let (answer, f) = flush_after_check(&mut s, &mut o, &t);
        assert_eq!(answer, ContractAnswer::Version(2));
        assert_eq!(f.accepted, 2);
        assert!(o.is_empty());
        let sent: Vec<serde_json::Value> = serde_json::from_str(&t.seen.borrow()[1]).unwrap();
        assert_eq!(sent[0]["contract_version"], 2, "積んだ版のまま送る");
    }

    #[test]
    fn st50_sender_transient_is_not_held() {
        for reply in [
            None,
            Some(Reply {
                status: 401,
                body: String::new(),
            }),
            Some(Reply {
                status: 403,
                body: String::new(),
            }),
            Some(Reply {
                status: 503,
                body: "<html>".into(),
            }),
        ] {
            assert_eq!(
                read_contract_answer(&reply.map_or_else(|| Err(anyhow::anyhow!("x")), Ok)),
                ContractAnswer::Transient
            );
        }
        let mut o: Outbox<IngestRequest> = Outbox::open(kept_tmp_path()).unwrap();
        o.add_many_at(vec![req(1)], 2).unwrap();
        let t = FakeTransport::default();
        t.contract.borrow_mut().push(None);
        let mut s = Sender::ingest();
        let (answer, f) = flush_after_check(&mut s, &mut o, &t);
        assert_eq!(answer, ContractAnswer::Transient);
        assert_eq!(f.sent, 0);
        assert_eq!(
            s.last_outcome(),
            SendOutcome::default(),
            "止まった状態になった"
        );
        assert_eq!(o.len(), 1);
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

    /// 途切れの問い合わせは**自分の利用者**で判定させる（review/code.md R13 / design D29（仮））。
    /// 付けないとサーバは既定の利用者で判定し、記録が届いているソースまで途切れで返る。
    #[test]
    fn outage_query_sends_device_and_user() {
        use crate::outage_notice::OutageQuery as _;
        let user = uuid::Uuid::parse_str("6f1c2a4e-0b7d-4e8a-9c3f-1d2e3f4a5b6c").unwrap();
        let body = serde_json::json!({"status": "warming_up", "judged_at": "2026-10-15T00:00:00Z"})
            .to_string();
        let (base, server) = one_shot_server("200 OK", body);
        let q = HttpOutageQuery::new(&base, "secret-token-0123456789", user);
        let _ = q.fetch("pc-1");
        let seen = server.join().unwrap();
        let line = seen.lines().next().unwrap_or_default();

        assert!(line.starts_with("GET /outages?"), "{line}");
        assert!(
            line.contains("device_id=pc-1"),
            "端末が付いていない: {line}"
        );
        assert!(
            line.contains(&format!("user_id={user}")),
            "利用者が付いていない: {line}"
        );
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
        let (_tmp, path) = tmp_path();
        let mut o: Outbox<IngestRequest> = Outbox::open(path).unwrap();
        o.add(req(1)).unwrap();
        let mut s = Sender::ingest();
        let mut lines = Vec::new();
        let mut log = |l: String| lines.push(l);
        let f = s.flush(&mut o, &t, Some(1), &mut log).unwrap();
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
