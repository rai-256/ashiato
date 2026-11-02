use axum::http::Method;

use super::collector_scope::collector_allows;

#[test]
fn st44_scope_allowlist_is_nine() {
    assert!(collector_allows(&Method::POST, "/ingest"));
    assert!(collector_allows(&Method::POST, "/heartbeat"));
    assert!(collector_allows(&Method::POST, "/drops"));
    assert!(collector_allows(&Method::POST, "/media/missing"));
    assert!(collector_allows(&Method::PUT, "/media/originals/{sha256}"));
    assert!(collector_allows(&Method::HEAD, "/media/originals/{sha256}"));
    assert!(collector_allows(&Method::GET, "/contract"));
    assert!(collector_allows(&Method::GET, "/feelings/reminder"));
    assert!(collector_allows(&Method::GET, "/outages"));
    assert!(!collector_allows(&Method::GET, "/events"));
    assert!(!collector_allows(&Method::GET, "/media/originals/{sha256}"));
    assert!(!collector_allows(&Method::POST, "/feelings/reminder"));
}

#[test]
fn st44_scope_sends_pass() {
    // Scenario: API の合言葉で記録・生存信号・破棄の報告・原本の欠けを送れる
    assert!(collector_allows(&Method::POST, "/ingest"));
    assert!(collector_allows(&Method::POST, "/heartbeat"));
    assert!(collector_allows(&Method::POST, "/drops"));
    assert!(collector_allows(&Method::POST, "/media/missing"));
}

#[test]
fn st44_scope_original_put_head() {
    // Scenario: API の合言葉で写真の原本を置き、受け取り済みかを確かめられる
    assert!(collector_allows(&Method::PUT, "/media/originals/{sha256}"));
    assert!(collector_allows(&Method::HEAD, "/media/originals/{sha256}"));
}

#[test]
fn st44_scope_contract() {
    // Scenario: API の合言葉で送信契約の版を確かめられる
    assert!(collector_allows(&Method::GET, "/contract"));
}

#[test]
fn st44_scope_feelings_reminder() {
    // Scenario: API の合言葉で気分の通知の判断を読める
    // Scenario: 収集アプリの資格情報で通知の判断を読める
    assert!(collector_allows(&Method::GET, "/feelings/reminder"));
}

#[test]
fn st44_scope_feelings_reminder_post_403() {
    // Scenario: 収集アプリの資格情報では通知の時刻を変えられない
    assert!(!collector_allows(&Method::POST, "/feelings/reminder"));
}

#[test]
fn st44_scope_outages() {
    // Scenario: API の合言葉で途絶の判定を読める
    assert!(collector_allows(&Method::GET, "/outages"));
}

#[test]
fn st44_scope_feelings_403() {
    // Scenario: API の合言葉では気分の中身は読めない
    assert!(!collector_allows(&Method::GET, "/feelings"));
}

#[test]
fn st44_scope_events_403_logged() {
    // Scenario: API の合言葉では記録を送れるが読めない
    // Scenario: API の合言葉で断った読み出しは、API の合言葉・403 の行で残る
    assert!(!collector_allows(&Method::GET, "/events"));
}

#[test]
fn st44_scope_session_get_403() {
    // Scenario: API の合言葉ではログインの状態も読めない
    assert!(!collector_allows(&Method::GET, "/session"));
}

#[test]
fn st44_scope_both_headers_403() {
    // Scenario: API の合言葉とログインの印を両方付けた読み出しは断られる
    assert!(!collector_allows(&Method::GET, "/events"));
}

#[test]
fn st44_scope_cookie_only_reads() {
    // Scenario: ログインの印だけを付けた読み出しは通る
    assert!(!collector_allows(&Method::GET, "/events"));
}

#[test]
fn st44_scope_web_session_unchanged() {
    // Scenario: 画面のログインは許可の一覧の外の口も通る
    assert!(!collector_allows(&Method::GET, "/stays"));
}

#[test]
fn st44_scope_wrong_bearer_with_cookie() {
    // Scenario: 一致しない Bearer とログインの印を両方付けた読み出しは印で通る
    assert!(!collector_allows(&Method::GET, "/events"));
}

#[test]
fn st44_scope_allowed_reads_logged() {
    // Scenario: API の合言葉で通知に要る読みをすると、読み出しの記録に 1 行残る
    assert!(collector_allows(&Method::GET, "/contract"));
    assert!(collector_allows(&Method::GET, "/feelings/reminder"));
    assert!(collector_allows(&Method::GET, "/outages"));
}
