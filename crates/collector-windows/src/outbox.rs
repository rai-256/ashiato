// SPDX-License-Identifier: AGPL-3.0-only
//! まだ送れていない記録の置き場（FR-10 / design D3）。
//!
//! **上限も破棄も持たない。** 上限を置く判断（`discarded`）は ST04 の担当で、
//! ST07 では扉を開けたままにする —— **捨てなければ何も失われない。**
//!
//! **S-01 と同じ PC で動いていても要る。** 網は切れないが、サーバの再起動・
//! 移行の適用・DB の停止は同じ PC でも起きる。ウィンドウは 1 日数千件出るので、
//! 数分の停止でも数百件になる。
//!
//! 形は C-01 の `FileOutboxStore` と同じ **追記のみの JSONL**。
//! **書きかけの行は読み飛ばして退避する** —— 1 行が壊れただけで
//! 残り全部を捨てるのは、捨てないという約束に反する。
use std::collections::HashMap;

use anyhow::Context as _;
use chrono::{DateTime, Utc};

use crate::contract::Outboxable;

/// 積んだ時刻の控えの 1 行（`<置き場>.enq.jsonl`。ST50 / final review R10）。
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Enqueued {
    id: uuid::Uuid,
    at: DateTime<Utc>,
}

/// 追記のみの JSONL に積む置き場。
#[derive(Debug)]
pub struct Outbox<T> {
    path: std::path::PathBuf,
    pending: Vec<T>,
    broken: usize,
    /// 行ごとの積んだ時刻（更新を促す通知の「未送信 n 日分」に使う。ST50 / design D5）。
    /// **行そのものには書かない** —— 行は送る形のままで、知らない欄は S-01 に送り返される（ST49 D3（仮））ので
    /// 欄を足すと `unknown_field` で断られる。控えに無い行（控えを書く前に積んだ・控えを書けなかった）は持たない
    enqueued: HashMap<uuid::Uuid, DateTime<Utc>>,
}

impl<T: Outboxable> Outbox<T> {
    /// 置き場を開き、**前回の続きを読み戻す**（起動と停止をまたいで保持する）。
    ///
    /// **無いときだけ空から始める。** 読めない（共有違反・権限・不正なバイト）ときは
    /// エラーにする —— 空として進むと、次の `remove` の書き直しが**溜まっていた全件を
    /// 上書きで消す**（review/code.md R20）。
    pub fn open(path: std::path::PathBuf) -> anyhow::Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).context("置き場を作れない")?;
        }
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(e).context("未送信の置き場を読めない"),
        };
        let mut pending = Vec::new();
        let mut broken = Vec::new();
        for l in text.lines().filter(|l| !l.trim().is_empty()) {
            match serde_json::from_str::<T>(l) {
                Ok(item) => pending.push(item),
                // **壊れた行は退避する。** 読めない 1 行のために残りを捨てない
                Err(_) => broken.push(l.to_string()),
            }
        }
        if !broken.is_empty() {
            // **退避先へは追記する**（上書きすると前回退避した行が消える。R30）
            use std::io::Write as _;
            let quarantine = path.with_extension("broken.jsonl");
            let mut f = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&quarantine)
                .context("退避先を開けない")?;
            for l in &broken {
                writeln!(f, "{l}").context("壊れた行を退避できない")?;
            }
            f.sync_all().context("退避先を同期できない")?;
        }
        let mut enqueued = HashMap::new();
        let mut enq_lines = 0usize;
        // 控えは日数の表示のためだけのもの。読めなければ持たないだけ（行は捨てない）
        if let Ok(text) = std::fs::read_to_string(enq_path(&path)) {
            for l in text.lines().filter(|l| !l.trim().is_empty()) {
                enq_lines += 1;
                if let Ok(e) = serde_json::from_str::<Enqueued>(l) {
                    enqueued.entry(e.id).or_insert(e.at);
                }
            }
        }
        let ids: std::collections::HashSet<uuid::Uuid> =
            pending.iter().map(Outboxable::id).collect();
        enqueued.retain(|id, _| ids.contains(id));
        let out = Self {
            path,
            pending,
            broken: broken.len(),
            enqueued,
        };
        if out.broken > 0 {
            out.rewrite()?;
        } else if enq_lines != out.enqueued.len() {
            // 送った行・壊れた行の控えが残っている。伸び続けないよう今の分だけに書き直す
            out.rewrite_enqueued();
        }
        Ok(out)
    }

    /// 行を積んだ時刻（控えに無ければ `None`）。
    pub fn enqueued_at(&self, id: &uuid::Uuid) -> Option<DateTime<Utc>> {
        self.enqueued.get(id).copied()
    }

    /// 開いたときに退避した壊れた行の数（**ログに出す**。黙って退避しない）。
    pub fn broken_lines(&self) -> usize {
        self.broken
    }

    /// 積む。**件数でも内容でもふるい落とさない。**
    pub fn add(&mut self, item: T) -> anyhow::Result<()> {
        self.add_many(vec![item])
    }

    /// まとめて積む。**追記をまとめ、同期は 1 回** —— 1 件ごとに同期すると、履歴の大きな取り込みの間
    /// 見回りが戻らない（deep.md 第 5 回 Q9。Windows 実機で 1 件約 1 ms）。
    /// 書けなかったものは溜まっているものに入れない（積み直す側が持つ）。
    pub fn add_many(&mut self, items: Vec<T>) -> anyhow::Result<()> {
        self.add_many_at(items, crate::contract::CONTRACT_VERSION)
    }

    /// 積む 1 か所。**送信契約の版をここで行に焼き込む**（ST50 / design D1）。
    /// 版を引数に取るのは、積んだ後に版が上がった状態を試験で作るため。
    pub fn add_many_at(&mut self, mut items: Vec<T>, version: u32) -> anyhow::Result<()> {
        for item in &mut items {
            item.burn_contract_version(version);
        }
        if items.is_empty() {
            return Ok(());
        }
        let mut text = String::new();
        for item in &items {
            text.push_str(&serde_json::to_string(item)?);
            text.push('\n');
        }
        // **追記する** —— 全件書き直しは未送信が伸びたときに書き込みを焼く
        use std::io::Write as _;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .context("置き場を開けない")?;
        f.write_all(text.as_bytes()).context("置き場へ書けない")?;
        // **同期する**（電源断で最後の追記が消えないように。R31）
        f.sync_data().context("置き場を同期できない")?;
        self.note_enqueued(&items, Utc::now());
        self.pending.extend(items);
        Ok(())
    }

    /// 積んだ時刻を控えに追記する。**書けなくても積んだことは取り消さない**（控えは日数の表示だけに使い、
    /// 無い行は出来事の時刻で代える）。書けなかった分はメモリには持つ。
    fn note_enqueued(&mut self, items: &[T], at: DateTime<Utc>) {
        let mut text = String::new();
        for item in items {
            self.enqueued.insert(item.id(), at);
            if let Ok(line) = serde_json::to_string(&Enqueued { id: item.id(), at }) {
                text.push_str(&line);
                text.push('\n');
            }
        }
        use std::io::Write as _;
        let _best_effort = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(enq_path(&self.path))
            .and_then(|mut f| f.write_all(text.as_bytes()));
    }

    /// 控えを今の行の分だけに書き直す（書けなくても行は残す。次に書き直すまで古い控えが残るだけ）。
    fn rewrite_enqueued(&self) {
        let mut text = String::new();
        for i in &self.pending {
            if let Some(at) = self.enqueued.get(&i.id()) {
                if let Ok(line) = serde_json::to_string(&Enqueued {
                    id: i.id(),
                    at: *at,
                }) {
                    text.push_str(&line);
                    text.push('\n');
                }
            }
        }
        let _best_effort = crate::fsutil::atomic_write(&enq_path(&self.path), text.as_bytes());
    }

    /// いま溜まっているもの。
    pub fn snapshot(&self) -> &[T] {
        &self.pending
    }

    /// 溜まっている件数。
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    /// 空か。
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// 送れたものだけを取り除く。**識別子で消す** ——
    /// 送信の途中で新しい記録が積まれても取り違えないため。
    pub fn remove(&mut self, ids: &[uuid::Uuid]) -> anyhow::Result<()> {
        if ids.is_empty() {
            return Ok(());
        }
        self.pending.retain(|i| !ids.contains(&i.id()));
        for id in ids {
            self.enqueued.remove(id);
        }
        self.rewrite()?;
        self.rewrite_enqueued();
        Ok(())
    }

    fn rewrite(&self) -> anyhow::Result<()> {
        let mut text = String::new();
        for i in &self.pending {
            text.push_str(&serde_json::to_string(i)?);
            text.push('\n');
        }
        // 一時ファイルに書いて同期してから置き換える —— 途中で落ちても
        // 「書きかけの全件」にならない
        crate::fsutil::atomic_write(&self.path, text.as_bytes())
    }
}

/// 積んだ時刻の控えの置き場（`outbox.jsonl` → `outbox.enq.jsonl`）。
fn enq_path(path: &std::path::Path) -> std::path::PathBuf {
    path.with_extension("enq.jsonl")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::contract::{IngestRequest, WindowPayload};

    fn tmp_dir() -> tempfile::TempDir {
        tempfile::Builder::new()
            .prefix("ashiato-outbox-")
            .tempdir()
            .unwrap()
    }

    fn req(n: u32) -> IngestRequest {
        let at = chrono::DateTime::parse_from_rfc3339("2026-09-13T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc)
            + chrono::Duration::seconds(n as i64);
        let mut p = WindowPayload::new(crate::contract::RecordKind::Foreground, at);
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

    /// **起動と停止をまたいで残る**（インスタンスの中だけに積まない。ST01 の実測）。
    #[test]
    fn outbox_survives_restart() {
        let tmp = tmp_dir();
        let dir = tmp.path().to_path_buf();
        let path = dir.join("outbox.jsonl");
        let a = req(1);
        let b = req(2);
        {
            let mut o: Outbox<IngestRequest> = Outbox::open(path.clone()).unwrap();
            o.add(a.clone()).unwrap();
            o.add(b.clone()).unwrap();
            assert_eq!(o.len(), 2);
        }
        let mut o: Outbox<IngestRequest> = Outbox::open(path.clone()).unwrap();
        assert_eq!(o.snapshot(), &[a.clone(), b.clone()]);
        o.remove(&[a.id()]).unwrap();
        let o: Outbox<IngestRequest> = Outbox::open(path).unwrap();
        assert_eq!(o.snapshot(), &[b], "取り除いた結果が置き場に残っていない");
    }

    /// 積んだ時刻は行に書かず控えに持ち、再起動をまたいで残り、取り除いた行の分は消える（final review R10）。
    /// 更新を促す通知の日数は「最も古い行を積んでからの日数」（design D5）で、出来事の時刻（古い訪問の履歴）ではない。
    // Scenario: 契約の版で止まると PC に通知が 1 回出る
    #[test]
    fn st50_outbox_keeps_enqueued_time_outside_the_row() {
        let tmp = tmp_dir();
        let path = tmp.path().join("outbox.jsonl");
        let before = Utc::now();
        let (a, b) = (req(1), req(2));
        {
            let mut o: Outbox<IngestRequest> = Outbox::open(path.clone()).unwrap();
            o.add_many(vec![a.clone(), b.clone()]).unwrap();
        }
        // 行は送る形のまま（控えの欄を混ぜると S-01 に知らない欄として送り返される）
        for line in std::fs::read_to_string(&path).unwrap().lines() {
            let row: serde_json::Value = serde_json::from_str(line).unwrap();
            let keys: Vec<&String> = row.as_object().unwrap().keys().collect();
            let sent: serde_json::Value = serde_json::to_value(&a).unwrap();
            assert_eq!(
                keys.len(),
                sent.as_object().unwrap().len(),
                "行に欄が増えた: {keys:?}"
            );
        }
        let mut o: Outbox<IngestRequest> = Outbox::open(path.clone()).unwrap();
        let at = o.enqueued_at(&a.id).expect("再起動で積んだ時刻が消えた");
        assert!(at >= before && at <= Utc::now());
        assert!(
            at > a.event_time.parse::<DateTime<Utc>>().unwrap(),
            "出来事の時刻を返した"
        );
        o.remove(&[a.id]).unwrap();
        assert_eq!(o.enqueued_at(&a.id), None);
        let o: Outbox<IngestRequest> = Outbox::open(path.clone()).unwrap();
        assert!(o.enqueued_at(&b.id).is_some());
        let enq = std::fs::read_to_string(path.with_extension("enq.jsonl")).unwrap();
        assert_eq!(
            enq.lines().count(),
            1,
            "取り除いた行の控えが残っている: {enq}"
        );
    }

    /// **壊れた 1 行のために残りを捨てない**（捨てないという約束を守る）。
    #[test]
    fn broken_line_is_quarantined_not_dropped() {
        let tmp = tmp_dir();
        let dir = tmp.path().to_path_buf();
        let path = dir.join("outbox.jsonl");
        let good = req(3);
        std::fs::write(
            &path,
            format!("{{書きかけ\n{}\n", serde_json::to_string(&good).unwrap()),
        )
        .unwrap();
        let o: Outbox<IngestRequest> = Outbox::open(path.clone()).unwrap();
        assert_eq!(o.snapshot(), std::slice::from_ref(&good));
        assert_eq!(o.broken_lines(), 1, "退避した件数が数えられていない");
        // 2 回目の退避が 1 回目を消さない（追記）
        std::fs::write(
            &path,
            format!("{{二つ目\n{}\n", serde_json::to_string(&good).unwrap()),
        )
        .unwrap();
        let _o: Outbox<IngestRequest> = Outbox::open(path).unwrap();
        let broken = std::fs::read_to_string(dir.join("outbox.broken.jsonl")).unwrap();
        assert!(
            broken.contains("書きかけ") && broken.contains("二つ目"),
            "前回退避した行が消えた: {broken}"
        );
    }

    /// まとめて積んだものは、1 件ずつ積んだものと同じく起動をまたいで残り、順も保つ（Q9）。
    #[test]
    fn add_many_survives_restart_in_order() {
        let tmp = tmp_dir();
        let dir = tmp.path().to_path_buf();
        let path = dir.join("outbox.jsonl");
        let items: Vec<_> = (1..=3).map(req).collect();
        {
            let mut o: Outbox<IngestRequest> = Outbox::open(path.clone()).unwrap();
            o.add(req(0)).unwrap();
            o.add_many(items.clone()).unwrap();
            o.add_many(Vec::new()).unwrap();
            assert_eq!(o.len(), 4);
        }
        let o: Outbox<IngestRequest> = Outbox::open(path).unwrap();
        assert_eq!(o.snapshot()[1..], items[..]);
    }

    /// **読めない置き場を空として開かない**（R20。空で開くと次の書き直しで全件消える）。
    #[test]
    fn unreadable_outbox_is_an_error_not_empty() {
        let tmp = tmp_dir();
        let dir = tmp.path().to_path_buf();
        let path = dir.join("outbox.jsonl");
        // 不正な UTF-8（読み取りが失敗する）
        std::fs::write(&path, [0xff, 0xfe, 0xfd, b'\n']).unwrap();
        assert!(Outbox::<IngestRequest>::open(path.clone()).is_err());
        assert_eq!(
            std::fs::read(&path).unwrap(),
            [0xff, 0xfe, 0xfd, b'\n'],
            "中身が書き換わった"
        );
    }

    /// 版の欄を持たない今の形の 1 行は、壊れた行にならず、版の欄の無いまま送る形で残る。
    ///
    /// Scenario: PC の版の欄を持たない未送信の行は、版の欄の無いまま送られる
    #[test]
    fn st49_c02_old_line_is_sent_without_version() {
        let dir = tmp_dir();
        let path = std::path::Path::join(dir.as_ref(), "outbox.jsonl");
        let line = r#"{"id":"00000000-0000-0000-0000-000000000001","user_id":"00000000-0000-0000-0000-000000000000","logical_source":"c02-window","external_id":null,"device_id":"dev-1","origin":"collected","event_time":"2026-09-13T00:00:00.000Z","tz_offset_min":540,"tz_id":"Asia/Tokyo","schema_version":1,"raw":"{}","payload":{}}"#;
        std::fs::write(&path, format!("{line}\n")).unwrap();
        let o: Outbox<IngestRequest> = Outbox::open(path).unwrap();
        assert_eq!(o.broken_lines(), 0, "壊れた行として退避された");
        assert_eq!(o.len(), 1);
        let sent = serde_json::to_value(&o.snapshot()[0]).unwrap();
        assert!(sent.get("client_version").is_none(), "{sent}");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// 今の C-02 の知らない欄を持つ行は、読み戻して書き直し・送る形にしても欄を落とさない。
    ///
    /// Scenario: PC の未送信の行が知らない欄を持っていても、送るときに落ちない
    #[test]
    fn st49_c02_unknown_field_survives_resend() {
        let dir = tmp_dir();
        let path = std::path::Path::join(dir.as_ref(), "outbox.jsonl");
        let mut v = serde_json::to_value(req(1)).unwrap();
        v["future_field"] = serde_json::json!({"n": 7});
        v["client_version"] = serde_json::json!("0.1.0+x");
        std::fs::write(&path, format!("{v}\n")).unwrap();
        let mut o: Outbox<IngestRequest> = Outbox::open(path.clone()).unwrap();
        let sent = serde_json::to_value(&o.snapshot()[0]).unwrap();
        assert_eq!(sent["future_field"], serde_json::json!({"n": 7}));
        assert_eq!(sent["client_version"], "0.1.0+x");
        // 別の行を取り除いて全件書き直しになっても残る
        o.add(req(2)).unwrap();
        o.remove(&[o.snapshot()[1].id]).unwrap();
        let o: Outbox<IngestRequest> = Outbox::open(path).unwrap();
        assert_eq!(
            serde_json::to_value(&o.snapshot()[0]).unwrap()["future_field"],
            serde_json::json!({"n": 7})
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
