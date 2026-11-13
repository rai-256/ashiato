// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
//! `setup` 副命令の保存と、保存直後の接続確認。

use crate::{
    config::Config,
    sender::{read_contract_answer, ContractAnswer, Transport},
};

pub fn destination_needs_confirmation(cfg: &Config, url: &str) -> anyhow::Result<bool> {
    let state = crate::destination_gate::Store::new(&cfg.state_dir).load()?;
    Ok(crate::destination_gate::gate(
        state.approved_url.as_deref(),
        url,
        crate::destination_gate::pending_in(&cfg.state_dir),
    ) == crate::destination_gate::GateAction::Ask)
}

pub fn approve_destination(cfg: &Config, url: &str) -> anyhow::Result<()> {
    let store = crate::destination_gate::Store::new(&cfg.state_dir);
    let mut state = store.load()?;
    state.approved_url = Some(url.to_owned());
    state.destination_notified_at = None;
    state.destination_notice_at = None;
    store.save(&state)
}

pub fn hold_destination(cfg: &Config, url: &str) -> anyhow::Result<()> {
    let store = crate::destination_gate::Store::new(&cfg.state_dir);
    let mut state = store.load()?;
    let _ = crate::destination_gate::answer(
        &mut state,
        url,
        Some(false),
        crate::destination_gate::pending_in(&cfg.state_dir),
        chrono::Utc::now(),
    );
    store.save(&state)
}

/// 読めない設定の文字列への答え（spec と一字一句）。
pub const UNREADABLE_TEXT: &str = "設定の文字列として読めません";
/// 何も貼らずに入力が閉じられたとき（EOF）の答え。読めない文字列と取り違えない（review/code.md R22）。
pub const CLOSED_TEXT: &str = "設定の文字列が入力されませんでした。何も保存していません";

/// 貼らせる流れの終わり方。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupOutcome {
    /// 保存した（か、貼らずに Enter で今の設定のまま保留を聞き直した）
    Saved,
    /// 読めない文字列だった。何も保存していない
    Unreadable,
    /// 入力が閉じられた（何も読めなかった）
    Closed,
}

/// 貼らせて保存し、確かめの結果を示し、未送信があれば y/N で送り先を聞く（spec の `setup` の流れ）。
///
/// `setup` 副命令と、設定の無い起動の窓（D14）の両方がこれを回す（review/code.md R2）。
/// `connect` は貼った送り先と資格情報で確かめの口を作る（試験は偽の口を渡す）。
pub fn run_setup_flow(
    cfg: &Config,
    input: &mut dyn std::io::BufRead,
    output: &mut dyn std::io::Write,
    connect: &dyn Fn(&str, &str) -> Box<dyn Transport>,
) -> anyhow::Result<SetupOutcome> {
    writeln!(output, "{}", crate::SETUP_PROMPT)?;
    output.flush()?;
    let mut line = String::new();
    if input.read_line(&mut line)? == 0 {
        return Ok(SetupOutcome::Closed);
    }
    let url = if line.trim().is_empty() {
        if cfg.needs_setup() {
            writeln!(output, "{UNREADABLE_TEXT}")?;
            return Ok(SetupOutcome::Unreadable);
        }
        cfg.base_url.clone()
    } else {
        let Some(parsed) = crate::setup_string::parse_setup_string(&line) else {
            writeln!(output, "{UNREADABLE_TEXT}")?;
            return Ok(SetupOutcome::Unreadable);
        };
        let transport = connect(&parsed.url, &parsed.token);
        let message = save_and_check(cfg, &line, &*transport)?;
        writeln!(output, "{message}")?;
        parsed.url
    };
    if destination_needs_confirmation(cfg, &url)? {
        let pending = crate::destination_gate::pending_summary(&cfg.state_dir);
        writeln!(
            output,
            "未送信 {} 件（{} 〜 {}）があります。この送り先へ送りますか [y/N]",
            pending.count,
            local_time(pending.oldest),
            local_time(pending.newest)
        )?;
        output.flush()?;
        let mut answer = String::new();
        input.read_line(&mut answer)?;
        if answer.trim().eq_ignore_ascii_case("y") {
            approve_destination(cfg, &url)?;
        } else {
            hold_destination(cfg, &url)?;
        }
    } else if !url.is_empty() {
        approve_destination(cfg, &url)?;
    }
    writeln!(output, "新しい版を確かめますか [Y/n]")?;
    output.flush()?;
    let mut update_answer = String::new();
    let enabled = if input.read_line(&mut update_answer)? == 0 {
        true
    } else {
        !update_answer.trim().eq_ignore_ascii_case("n")
    };
    crate::update_check::UpdateChecker::new(&cfg.state_dir).set_enabled(enabled)?;
    output.flush()?;
    Ok(SetupOutcome::Saved)
}

/// y/N に見せる時刻は手元の時刻で（review/code.md R14）。
fn local_time(at: Option<chrono::DateTime<chrono::Utc>>) -> String {
    at.map_or_else(
        || "不明".to_owned(),
        |t| {
            t.with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M")
                .to_string()
        },
    )
}

pub fn save_and_check(
    cfg: &Config,
    input: &str,
    transport: &dyn Transport,
) -> anyhow::Result<&'static str> {
    cfg.save_setup_string(input)?;
    let reply = transport.get("/contract");
    // 確かめの答えも「届いた」に数える（保留のまま 24 時間たっても「届かない」と誤って知らせない。review/code.md R21）
    if let Ok(status) = reply.as_ref().map(|r| r.status) {
        let store = crate::destination_gate::Store::new(&cfg.state_dir);
        let mut state = store.load()?;
        crate::destination_gate::observe_response(&mut state, Some(status), chrono::Utc::now());
        store.save(&state)?;
    }
    let answer = read_contract_answer(&reply);
    Ok(match answer {
        ContractAnswer::Version(_) => {
            let server_is_dev = crate::sender::read_contract_channel(&reply)
                == Some(ashiato_build_stamp::Channel::Dev);
            let client_is_dev = cfg.device_id.ends_with("-dev");
            if server_is_dev == client_is_dev {
                "繋がりました"
            } else if server_is_dev {
                "送り先が開発版のサーバです"
            } else {
                "送り先が正式版のサーバです"
            }
        }
        ContractAnswer::Transient if reply.as_ref().is_ok_and(|r| r.status == 401) => {
            "資格情報か送り先が違います"
        }
        _ => "保存しました。繋がったら送ります",
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::sender::Reply;
    use std::cell::RefCell;

    #[derive(Debug)]
    struct Fake {
        reply: Option<Reply>,
        paths: RefCell<Vec<String>>,
    }
    impl Transport for Fake {
        fn post(&self, path: &str, _: &str, _: u32) -> anyhow::Result<Reply> {
            self.paths.borrow_mut().push(path.into());
            unreachable!()
        }
        fn get(&self, path: &str) -> anyhow::Result<Reply> {
            self.paths.borrow_mut().push(path.into());
            self.reply.clone().ok_or_else(|| anyhow::anyhow!("offline"))
        }
    }
    fn cfg() -> (Config, tempfile::TempDir) {
        let tmp = tempfile::tempdir().unwrap();
        (
            Config {
                base_url: String::new(),
                api_token: String::new(),
                user_id: uuid::Uuid::nil(),
                device_id: "pc-01".into(),
                state_dir: tmp.path().into(),
                settings_mtime: None,
                credential_mtime: None,
                config_from_env: false,
                exclusions_path: tmp.path().join("exclusions.json"),
                launch_state_dir: tmp.path().into(),
                launch_device_id: "pc-01".into(),
            },
            tmp,
        )
    }
    fn input() -> &'static str {
        "ashiato-collector:1:eyJ1cmwiOiJodHRwczovL3BjLmV4YW1wbGUudHMubmV0L2FwaSIsInRva2VuIjoidG9rZW4tZm9yLXRlc3QtMDEyMzQ1Njc4OSJ9"
    }
    fn reply(status: u16, body: &str) -> Option<Reply> {
        Some(Reply {
            status,
            body: body.into(),
            headers: Default::default(),
        })
    }

    // Scenario: 401 でも保存して結果を示す
    #[test]
    fn st56_setupcmd_401_is_saved_and_reported() {
        let (c, _tmp) = cfg();
        let t = Fake {
            reply: reply(401, ""),
            paths: RefCell::new(vec![]),
        };
        assert_eq!(
            save_and_check(&c, input(), &t).unwrap(),
            "資格情報か送り先が違います"
        );
        assert!(c.state_dir.join("credential.bin").exists());
    }
    // Scenario: setup で貼った設定の文字列を保存する
    #[test]
    fn st56_setupcmd_pasted_setup_string_is_saved() {
        let (c, _tmp) = cfg();
        let t = Fake {
            reply: reply(200, r#"{"contract_version":1}"#),
            paths: RefCell::new(vec![]),
        };
        assert_eq!(save_and_check(&c, input(), &t).unwrap(), "繋がりました");
        assert!(c.state_dir.join("settings.json").exists());
        assert!(c.state_dir.join("credential.bin").exists());
    }
    // Scenario: 届かなくても保存する
    #[test]
    fn st56_setupcmd_unreachable_is_saved_and_reported() {
        let (c, _tmp) = cfg();
        let t = Fake {
            reply: None,
            paths: RefCell::new(vec![]),
        };
        assert_eq!(
            save_and_check(&c, input(), &t).unwrap(),
            "保存しました。繋がったら送ります"
        );
        assert!(c.state_dir.join("settings.json").exists());
    }
    // Scenario: PC で繋がれば「繋がりました」
    #[test]
    fn st56_setupcmd_connected_is_reported() {
        let (c, _tmp) = cfg();
        let t = Fake {
            reply: reply(200, r#"{"contract_version":1}"#),
            paths: RefCell::new(vec![]),
        };
        assert_eq!(save_and_check(&c, input(), &t).unwrap(), "繋がりました");
    }
    // Scenario: PC で系統の違うサーバなら、その旨を出す
    #[test]
    fn st56_setupcmd_dev_server_is_reported() {
        let (c, _tmp) = cfg();
        let t = Fake {
            reply: Some(Reply {
                status: 200,
                body: r#"{"contract_version":1}"#.into(),
                headers: [("Ashiato-Channel".to_string(), "dev".to_string())].into(),
            }),
            paths: RefCell::new(vec![]),
        };
        assert_eq!(
            save_and_check(&c, input(), &t).unwrap(),
            "送り先が開発版のサーバです"
        );
    }

    #[test]
    fn st56_setupcmd_dev_client_accepts_dev_channel() {
        let (mut c, _tmp) = cfg();
        c.device_id = "pc-01-dev".into();
        let t = Fake {
            reply: Some(Reply {
                status: 200,
                body: r#"{"contract_version":1}"#.into(),
                headers: [("Ashiato-Channel".to_string(), "dev".to_string())].into(),
            }),
            paths: RefCell::new(vec![]),
        };
        assert_eq!(save_and_check(&c, input(), &t).unwrap(), "繋がりました");
    }
    // Scenario: PC の確かめの問い合わせでは記録を送らない
    #[test]
    fn st56_setupcmd_check_only_reads_contract() {
        let (c, _tmp) = cfg();
        let t = Fake {
            reply: reply(200, r#"{"contract_version":1}"#),
            paths: RefCell::new(vec![]),
        };
        save_and_check(&c, input(), &t).unwrap();
        assert_eq!(&*t.paths.borrow(), &["/contract"]);
    }
    fn flow(c: &Config, typed: &str) -> (SetupOutcome, String) {
        let mut input = std::io::Cursor::new(typed.as_bytes().to_vec());
        let mut output = Vec::new();
        let connect = |_: &str, _: &str| -> Box<dyn Transport> {
            Box::new(Fake {
                reply: reply(200, r#"{"contract_version":1}"#),
                paths: RefCell::new(vec![]),
            })
        };
        let outcome = run_setup_flow(c, &mut input, &mut output, &connect).unwrap();
        (outcome, String::from_utf8(output).unwrap())
    }

    fn approved_a_with_pending(c: &Config) {
        std::fs::create_dir_all(&c.state_dir).unwrap();
        crate::destination_gate::Store::new(&c.state_dir)
            .save(&crate::destination_gate::State {
                approved_url: Some("https://a.example/api".into()),
                ..Default::default()
            })
            .unwrap();
        std::fs::write(
            c.state_dir.join("heartbeat.jsonl"),
            "{\"emitted_at\":\"2026-11-01T00:00:00Z\"}\n",
        )
        .unwrap();
    }

    #[test]
    fn st56_setupcmd_window_flow_asks_and_holds_on_no() {
        // review/code.md R2: 窓の経路も同じ流れ（結果の文 → y/N → 保留と知らせ）
        let (c, _tmp) = cfg();
        approved_a_with_pending(&c);
        let (outcome, out) = flow(&c, &format!("{}\nN\n", input()));
        assert_eq!(outcome, SetupOutcome::Saved);
        assert!(out.contains(crate::SETUP_PROMPT), "{out}");
        assert!(out.contains("繋がりました"), "{out}");
        assert!(out.contains("未送信 1 件"), "{out}");
        assert!(out.contains("[y/N]"), "{out}");
        assert!(!out.contains("token-for-test-0123456789"));
        let state = crate::destination_gate::Store::new(&c.state_dir)
            .load()
            .unwrap();
        assert_eq!(state.approved_url.as_deref(), Some("https://a.example/api"));
        assert!(
            state.destination_notified_at.is_some(),
            "保留の知らせが立たない"
        );
        assert!(crate::destination_gate::destination_notice(
            &state,
            chrono::Utc::now()
        ));
    }

    #[test]
    fn st56_setupcmd_window_flow_approves_on_yes() {
        let (c, _tmp) = cfg();
        approved_a_with_pending(&c);
        let (_, out) = flow(&c, &format!("{}\ny\n", input()));
        assert!(out.contains("[y/N]"), "{out}");
        let state = crate::destination_gate::Store::new(&c.state_dir)
            .load()
            .unwrap();
        assert_eq!(
            state.approved_url.as_deref(),
            Some("https://pc.example.ts.net/api")
        );
        assert!(state.destination_notified_at.is_none());
    }

    #[test]
    fn st56_setupcmd_window_flow_closed_without_answer_still_notifies() {
        // 答えずに窓を閉じても保留の知らせが出る（spec）。y/N を聞く前の保存の時点で保留に入る
        let (c, _tmp) = cfg();
        approved_a_with_pending(&c);
        c.save_setup_string(input()).unwrap();
        let state = crate::destination_gate::Store::new(&c.state_dir)
            .load()
            .unwrap();
        assert_eq!(state.approved_url.as_deref(), Some("https://a.example/api"));
        assert!(state.destination_notified_at.is_some());
    }

    #[test]
    fn st56_setupcmd_window_flow_rejects_unreadable_text() {
        let (c, _tmp) = cfg();
        let (outcome, out) = flow(&c, "hello\n");
        assert_eq!(outcome, SetupOutcome::Unreadable);
        assert!(out.contains(UNREADABLE_TEXT), "{out}");
        assert!(!c.state_dir.join("settings.json").exists());
        assert_eq!(flow(&c, "").0, SetupOutcome::Closed);
    }

    #[test]
    fn st56_setupcmd_save_resets_counts_and_starts_unreachable_clock() {
        // review/code.md R10: 保存し直すと数えをやめ、「届かない」は保存の時刻から数える
        let (c, _tmp) = cfg();
        std::fs::create_dir_all(&c.state_dir).unwrap();
        let t0 = chrono::DateTime::parse_from_rfc3339("2026-11-01T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        crate::destination_gate::Store::new(&c.state_dir)
            .save(&crate::destination_gate::State {
                auth_since: Some(t0),
                auth_notified_at: Some(t0),
                first_response_at: Some(t0),
                unreachable_since: Some(t0),
                unreachable_notified_at: Some(t0),
                ..Default::default()
            })
            .unwrap();
        let saved = t0 + chrono::Duration::hours(5);
        c.save_setup_string_at(input(), saved).unwrap();
        let state = crate::destination_gate::Store::new(&c.state_dir)
            .load()
            .unwrap();
        assert_eq!(state.setup_saved_at, Some(saved));
        assert_eq!(state.auth_since, None);
        assert_eq!(state.auth_notified_at, None);
        assert_eq!(state.first_response_at, None);
        assert_eq!(state.unreachable_notified_at, None);
        assert_eq!(
            crate::destination_gate::notice_kind(
                &state,
                saved + chrono::Duration::minutes(24 * 60 - 1)
            ),
            None
        );
        assert_eq!(
            crate::destination_gate::notice_kind(
                &state,
                saved + chrono::Duration::minutes(24 * 60 + 1)
            ),
            Some(crate::destination_gate::NoticeKind::Unreachable)
        );
    }

    // Scenario: 戻せない資格情報は未設定として扱う
    #[test]
    fn st56_setupcmd_unreadable_credential_is_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let state = tmp.path().join("ashiato/collector");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(
            state.join("settings.json"),
            r#"{"base_url":"https://configured.example","device_id":"pc-01"}"#,
        )
        .unwrap();
        std::fs::write(state.join("credential.bin"), b"bad").unwrap();
        let env = |k: &str| {
            Some(match k {
                "LOCALAPPDATA" => tmp.path().to_string_lossy().into_owned(),
                _ => return None,
            })
        };
        let loaded = crate::config::Config::from_sources(
            &[],
            env,
            None,
            ashiato_build_stamp::Channel::Official,
        )
        .unwrap();
        assert!(loaded.needs_setup());
        assert!(loaded.base_url.contains("configured.example"));
    }
}
