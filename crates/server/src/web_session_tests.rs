// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
//! 画面のログイン（`/session`）を、router を通して本物の DB に対して確かめる（ST28 / design D1〜D3 / D17 / D18）。
//! 合言葉は試験の中で乱数から作る。
#![allow(clippy::unwrap_used)]

use axum::body::Body;
use axum::http::{header, Method, Request, Response};
use tower::ServiceExt as _;

use super::*;
use crate::testdb;

const TOKEN: &str = "test-token-0123456789abcdef";

#[test]
fn st44_session_key_validation_rejects_missing_short_and_duplicates() {
    assert_eq!(
        web_session::check_web_session_key(None, TOKEN, "web-password-0123456789"),
        Err(web_session::WebSessionKeyRefusal::Missing)
    );
    assert_eq!(
        web_session::check_web_session_key(Some("short"), TOKEN, "web-password-0123456789"),
        Err(web_session::WebSessionKeyRefusal::TooShort)
    );
    assert_eq!(
        web_session::check_web_session_key(Some(TOKEN), TOKEN, "web-password-0123456789"),
        Err(web_session::WebSessionKeyRefusal::SameAsApiToken)
    );
    assert_eq!(
        web_session::check_web_session_key(
            Some("web-password-0123456789"),
            TOKEN,
            "web-password-0123456789"
        ),
        Err(web_session::WebSessionKeyRefusal::SameAsWebPassword)
    );
    // 下限 16 文字の境界の両側（spec の「15 文字の値」。code_verify R18）
    assert_eq!(
        web_session::check_web_session_key(
            Some("session-key-015"),
            TOKEN,
            "web-password-0123456789"
        ),
        Err(web_session::WebSessionKeyRefusal::TooShort)
    );
    assert_eq!(
        web_session::check_web_session_key(
            Some("session-key-0016"),
            TOKEN,
            "web-password-0123456789"
        ),
        Ok(())
    );
}

#[test]
fn st44_credential_present_counts_only_bearer_and_session_cookie() {
    use axum::http::{header, HeaderMap, HeaderValue};
    let h = |pairs: &[(header::HeaderName, &'static str)]| {
        let mut m = HeaderMap::new();
        for (k, v) in pairs {
            m.append(k.clone(), HeaderValue::from_static(v));
        }
        m
    };
    assert!(!crate::credential_present(&h(&[]), Channel::Official));
    // 同じ host のほかの cookie と Bearer でない見出しは数えない（/code-review R23）
    assert!(!crate::credential_present(
        &h(&[(header::COOKIE, "ts_session=abc; other=1")]),
        Channel::Official
    ));
    assert!(!crate::credential_present(
        &h(&[(header::AUTHORIZATION, "Basic dXNlcjpwYXNz")]),
        Channel::Official
    ));
    assert!(!crate::credential_present(
        &h(&[(header::AUTHORIZATION, "Bearer ")]),
        Channel::Official
    ));
    assert!(crate::credential_present(
        &h(&[(header::AUTHORIZATION, "Bearer wrong-token")]),
        Channel::Official
    ));
    assert!(crate::credential_present(
        &h(&[(header::COOKIE, "other=1; ashiato_session=expired")]),
        Channel::Official
    ));
}

#[test]
fn st44_secret_tag_uses_session_key() {
    assert_ne!(
        web_session::secret_tag("api-one-0123456789", "password-0123456789"),
        web_session::secret_tag("api-two-0123456789", "password-0123456789")
    );
}

fn random_password() -> String {
    format!("pw-{}", uuid::Uuid::new_v4())
}

async fn app_with(password: &str, days: u32) -> App {
    App::for_test(testdb::pool().await, TOKEN).with_web_login(password, days)
}

async fn send(app: &App, req: Request<Body>) -> Response<Body> {
    router(app.clone()).oneshot(req).await.unwrap()
}

fn login_req(body: &str) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri("/session")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_owned()))
        .unwrap()
}

fn login_body(password: &str) -> String {
    serde_json::json!({ "password": password }).to_string()
}

fn with_cookie(method: Method, uri: &str, cookie: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::COOKIE, format!("{COOKIE}={cookie}"))
        .body(Body::empty())
        .unwrap()
}

const COOKIE: &str = web_session::COOKIE_NAME;

fn set_cookie(res: &Response<Body>) -> Option<String> {
    res.headers()
        .get(header::SET_COOKIE)
        .map(|v| v.to_str().unwrap().to_owned())
}

/// `Set-Cookie` から印の値を取り出す。
fn token_of(set_cookie: &str) -> String {
    set_cookie
        .strip_prefix(&format!("{COOKIE}="))
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned()
}

fn max_age(set_cookie: &str) -> u64 {
    set_cookie
        .split(';')
        .find_map(|p| p.trim().strip_prefix("Max-Age="))
        .unwrap()
        .parse()
        .unwrap()
}

/// ログインして印の値を返す。
async fn login(app: &App, password: &str) -> String {
    let res = send(app, login_req(&login_body(password))).await;
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    token_of(&set_cookie(&res).unwrap())
}

async fn read_events(app: &App, cookie: &str) -> Response<Body> {
    send(app, with_cookie(Method::GET, "/events", cookie)).await
}

/// Scenario: 違う合言葉のログインの求めは断られ、印は発行されない
#[tokio::test]
async fn web_session_endpoint_wrong_password_is_refused() {
    let pw = random_password();
    let app = app_with(&pw, 0).await;
    let res = send(&app, login_req(&login_body(&random_password()))).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    assert!(set_cookie(&res).is_none(), "印が発行された");
}

/// Scenario: 合言葉を付けないログインの求めは断られる
#[tokio::test]
async fn web_session_endpoint_missing_password_is_refused() {
    let app = app_with(&random_password(), 0).await;
    for body in ["{}", "", "not json", r#"{"password":null}"#] {
        let res = send(&app, login_req(body)).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED, "本文: {body}");
        assert!(set_cookie(&res).is_none(), "印が発行された（本文: {body}）");
    }
}

/// Scenario: API の合言葉では画面にログインできない
#[tokio::test]
async fn web_session_endpoint_api_token_is_not_a_login() {
    let app = app_with(&random_password(), 0).await;
    let res = send(&app, login_req(&login_body(TOKEN))).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    assert!(set_cookie(&res).is_none());
}

/// Scenario: ログインの印は暗号化された接続でだけ送られる
#[tokio::test]
async fn web_session_endpoint_cookie_is_secure() {
    let pw = random_password();
    let app = app_with(&pw, 0).await;
    let res = send(&app, login_req(&login_body(&pw))).await;
    let sc = set_cookie(&res).unwrap();
    assert!(sc.split("; ").any(|a| a == "Secure"), "{sc}");
    assert!(sc.split("; ").any(|a| a == "HttpOnly"));
    assert!(!sc.contains("Domain"));
}

/// Scenario: ログインの印は別のサイトから始まった求めには付かない
#[tokio::test]
async fn web_session_endpoint_cookie_is_same_site_strict() {
    let pw = random_password();
    let app = app_with(&pw, 0).await;
    let res = send(&app, login_req(&login_body(&pw))).await;
    let sc = set_cookie(&res).unwrap();
    assert!(sc.split("; ").any(|a| a == "SameSite=Strict"), "{sc}");
}

/// Scenario: ログアウトした印を持ち出しても使えない
#[tokio::test]
async fn web_session_endpoint_logged_out_cookie_is_dead() {
    let pw = random_password();
    let app = app_with(&pw, 0).await;
    let token = login(&app, &pw).await;
    assert_eq!(read_events(&app, &token).await.status(), StatusCode::OK);

    let out = send(&app, with_cookie(Method::DELETE, "/session", &token)).await;
    assert_eq!(out.status(), StatusCode::NO_CONTENT);
    assert_eq!(max_age(&set_cookie(&out).unwrap()), 0);
    // 印が無くても 204
    let none = send(
        &app,
        Request::builder()
            .method(Method::DELETE)
            .uri("/session")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(none.status(), StatusCode::NO_CONTENT);

    let again = read_events(&app, &token).await;
    assert_eq!(again.status(), StatusCode::UNAUTHORIZED);
    let state = send(&app, with_cookie(Method::GET, "/session", &token)).await;
    assert_eq!(state.status(), StatusCode::UNAUTHORIZED);
}

/// Scenario: 失敗を重ねたログインは一時的に断られる
#[tokio::test]
async fn web_session_endpoint_failures_are_throttled() {
    let pw = random_password();
    let app = app_with(&pw, 0).await;
    for _ in 0..10 {
        let res = send(&app, login_req(&login_body(&random_password()))).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
    let res = send(&app, login_req(&login_body(&pw))).await;
    assert_eq!(res.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(set_cookie(&res).is_none(), "印が発行された");
}

#[tokio::test]
async fn web_session_endpoint_failure_waits_and_state_reports_caller() {
    let pw = random_password();
    let mut app = app_with(&pw, 0).await;
    app.login.failure_delay = std::time::Duration::from_millis(300);
    let t = std::time::Instant::now();
    send(&app, login_req(&login_body("x"))).await;
    assert!(t.elapsed() >= std::time::Duration::from_millis(300));

    let token = login(&app, &pw).await;
    let by_cookie = send(&app, with_cookie(Method::GET, "/session", &token)).await;
    let body = axum::body::to_bytes(by_cookie.into_body(), 4096)
        .await
        .unwrap();
    assert_eq!(&body[..], br#"{"credential":"web_session"}"#);
    let by_bearer = send(
        &app,
        Request::builder()
            .uri("/session")
            .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    // ST44 の許可の一覧にないため、API の合言葉によるセッション読み出しは403。
    assert_eq!(by_bearer.status(), StatusCode::FORBIDDEN);
}

/// Scenario: API の合言葉ではログインの状態も読めない
#[tokio::test]
async fn web_session_endpoint_api_token_cannot_read_session() {
    let app = app_with(&random_password(), 0).await;
    let bearer = format!("Bearer {TOKEN}");
    // ST44 の許可の一覧にないため、API の合言葉によるセッション読み出しは403。
    let get = Request::builder()
        .uri("/session")
        .header(header::AUTHORIZATION, &bearer)
        .body(Body::empty())
        .unwrap();
    let res = send(&app, get).await;
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

/// Scenario: 既定では日が経ってもログインは切れない
#[tokio::test]
async fn web_session_lifetime_default_never_expires() {
    let pw = random_password();
    let now = chrono::Utc::now();
    // 「期限を設定せずに」= 環境に WEB_SESSION_MAX_AGE_DAYS が無いときの既定（code-verify R15）
    let days = web_session::session_max_age_days(None).unwrap();
    let app = app_with(&pw, days).await.at(now);
    let token = login(&app, &pw).await;
    let later = app.clone().at(now + chrono::Duration::days(400));
    assert_eq!(read_events(&later, &token).await.status(), StatusCode::OK);
}

/// Scenario: ログインの印はブラウザを閉じても残り、使うたびに延びる
#[tokio::test]
async fn web_session_lifetime_cookie_persists_and_is_refreshed() {
    let pw = random_password();
    let app = app_with(&pw, 0).await;
    let res = send(&app, login_req(&login_body(&pw))).await;
    let login_cookie = set_cookie(&res).unwrap();
    assert_eq!(max_age(&login_cookie), 400 * 86_400, "{login_cookie}");
    let token = token_of(&login_cookie);

    let read = read_events(&app, &token).await;
    assert_eq!(read.status(), StatusCode::OK);
    let read_cookie = set_cookie(&read).expect("読み出しの応答に印の出し直しが無い");
    assert_eq!(max_age(&read_cookie), 400 * 86_400, "{read_cookie}");
    assert_eq!(token_of(&read_cookie), token);
}

/// Scenario: 画面の合言葉を変えると、それまでのログインはすべて使えなくなる
#[tokio::test]
async fn web_session_lifetime_password_change_revokes_all() {
    let pw = random_password();
    let app = app_with(&pw, 0).await;
    let a = login(&app, &pw).await;
    let b = login(&app, &pw).await;
    assert_eq!(read_events(&app, &a).await.status(), StatusCode::OK);

    let restarted = app.clone().with_web_login(&random_password(), 0);
    assert_eq!(
        read_events(&restarted, &a).await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        read_events(&restarted, &b).await.status(),
        StatusCode::UNAUTHORIZED
    );
}

/// Scenario: 期限を設定したときは、過ぎた印は使えない
#[tokio::test]
async fn web_session_lifetime_configured_expiry_applies() {
    let pw = random_password();
    let now = chrono::Utc::now();
    let app = app_with(&pw, 30).await.at(now);
    let token = login(&app, &pw).await;
    let inside = app.clone().at(now + chrono::Duration::days(29));
    assert_eq!(read_events(&inside, &token).await.status(), StatusCode::OK);
    let outside = app.clone().at(now + chrono::Duration::days(31));
    assert_eq!(
        read_events(&outside, &token).await.status(),
        StatusCode::UNAUTHORIZED
    );
}

/// 有効な印を持ったまま合言葉でログインし直すと、`Set-Cookie` は新しい印の 1 本だけ
/// （古い印の出し直しが後に並ぶと、ブラウザは後のほうを採る。final review R2）。
#[tokio::test]
async fn web_session_endpoint_relogin_with_cookie_sets_only_new_cookie() {
    let pw = random_password();
    let app = app_with(&pw, 0).await;
    let old = login(&app, &pw).await;
    let req = Request::builder()
        .method(Method::POST)
        .uri("/session")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, format!("{COOKIE}={old}"))
        .body(Body::from(login_body(&pw)))
        .unwrap();
    let res = send(&app, req).await;
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    let all: Vec<_> = res.headers().get_all(header::SET_COOKIE).iter().collect();
    assert_eq!(all.len(), 1, "Set-Cookie が 1 本でない");
    let new = token_of(all[0].to_str().unwrap());
    assert_ne!(new, old, "新しい印が渡っていない");
    assert_eq!(read_events(&app, &new).await.status(), StatusCode::OK);
}

/// 認可の判定は 1 求めに 1 回（final review R3）。外側の層が下した判定があれば、ハンドラの
/// `authorize` は DB を引き直さずにそれを使う —— 途中でログアウトが割り込んでも、記録（ok / 200）と
/// 応答が食い違わない。
#[tokio::test]
async fn web_session_authorization_is_decided_once_per_request() {
    let pw = random_password();
    let app = app_with(&pw, 0).await;
    let token = login(&app, &pw).await;
    let headers = with_cookie(Method::GET, "/events", &token)
        .headers()
        .clone();
    let decided = crate::decide(&app, &headers).await;
    assert_eq!(
        decided,
        Authn::Decided(Some(web_session::Caller::WebSession))
    );

    // 判定の後にログアウトしても、この求めの中では同じ判定が使われる
    let res = send(&app, with_cookie(Method::DELETE, "/session", &token)).await;
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    let inner = crate::AUTHN
        .scope(decided, async { authorize(&app, &headers).await })
        .await;
    assert_eq!(inner.unwrap(), web_session::Caller::WebSession);
    // 判定の外（次の求め）では、ログアウトが効いている
    assert_eq!(
        authorize(&app, &headers).await.unwrap_err().0,
        StatusCode::UNAUTHORIZED
    );
}

// ---- CH15: 照らす形のサーバで、資格情報を返す口が正しい合言葉を受け付ける ----

mod ch15 {
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use axum::body::Body;
    use axum::http::{header, Method, Request, StatusCode};
    use http_body_util::BodyExt as _;
    use tower::ServiceExt as _;

    use crate::access_log::{AccessEntry, AccessFuture, AccessSink};
    use crate::testdb::{self, VERIFIER_APP_TOKEN as API_TOKEN};
    use crate::{web_session, App};

    const PASSWORD: &str = "install-password-012345";

    async fn verifier_app() -> App {
        testdb::verifier_app(PASSWORD, Duration::ZERO).await
    }

    /// 書く口に届いた行を残すだけの偽物（並走する試験の行と混ざらないように）。
    #[derive(Clone, Default)]
    struct Recorder(Arc<Mutex<Vec<AccessEntry>>>);

    impl AccessSink for Recorder {
        fn record(&self, entry: AccessEntry) -> AccessFuture {
            self.0.lock().unwrap().push(entry);
            Box::pin(async { Ok(()) })
        }
    }

    impl Recorder {
        fn count(&self, route: &str, outcome: &str) -> usize {
            self.0
                .lock()
                .unwrap()
                .iter()
                .filter(|e| e.route == route && e.outcome == outcome)
                .count()
        }
    }

    fn json_post(uri: &str, password: &str) -> Request<Body> {
        Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                serde_json::json!({ "password": password }).to_string(),
            ))
            .unwrap()
    }

    fn setup_request(cookie: &str, password: &str) -> Request<Body> {
        let mut req = json_post("/collector-setup", password);
        req.headers_mut()
            .insert(header::COOKIE, cookie.parse().unwrap());
        req
    }

    async fn setup(app: &App, cookie: &str, password: &str) -> (StatusCode, String, String) {
        let res = crate::router(app.clone())
            .oneshot(setup_request(cookie, password))
            .await
            .unwrap();
        let status = res.status();
        let cache = res
            .headers()
            .get(header::CACHE_CONTROL)
            .map(|v| v.to_str().unwrap().to_owned())
            .unwrap_or_default();
        let body = res.into_body().collect().await.unwrap().to_bytes();
        (status, cache, String::from_utf8(body.to_vec()).unwrap())
    }

    #[tokio::test]
    async fn ch15_verifier_collector_setup_returns_token() {
        // Scenario: 合言葉を入れ直すと、設定の文字列とコピーのボタンが出る
        let app = verifier_app().await;
        let cookie = testdb::web_cookie(&app, PASSWORD).await;
        let (status, cache, body) = setup(&app, &cookie, PASSWORD).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(cache, "no-store");
        let value: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(value, serde_json::json!({ "token": API_TOKEN }));
    }

    #[tokio::test]
    async fn ch15_verifier_collector_setup_wrong_password_is_forbidden() {
        // Scenario: 合言葉が違えば出さない
        let app = verifier_app().await;
        let cookie = testdb::web_cookie(&app, PASSWORD).await;
        for given in ["install-password-012346", ""] {
            let (status, _, body) = setup(&app, &cookie, given).await;
            assert_eq!(status, StatusCode::FORBIDDEN, "given={given:?}");
            let value: serde_json::Value = serde_json::from_str(&body).unwrap();
            assert_eq!(value, serde_json::json!({ "error": "wrong_password" }));
            assert!(!body.contains(API_TOKEN));
        }
        // ログインの印は有効なまま（正しい合言葉で答える）
        let (status, _, _) = setup(&app, &cookie, PASSWORD).await;
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn ch15_verifier_collector_setup_shares_login_failure_limit() {
        // Scenario: 失敗が回数の制限を超えると、正しい合言葉でも答えない
        let app = verifier_app().await;
        let cookie = testdb::web_cookie(&app, PASSWORD).await;
        for _ in 0..9 {
            let (status, _, _) = setup(&app, &cookie, "wrong-password-0123456789").await;
            assert_eq!(status, StatusCode::FORBIDDEN);
        }
        let failed = crate::router(app.clone())
            .oneshot(json_post("/session", "wrong-login-0123456789"))
            .await
            .unwrap();
        assert_eq!(failed.status(), StatusCode::UNAUTHORIZED);
        let (status, _, _) = setup(&app, &cookie, PASSWORD).await;
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
        let (status, _, _) = setup(&app, &cookie, "").await;
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    }

    /// 空の合言葉も、資格情報の口で失敗に数える（design D1。review/code.md R3）。
    #[tokio::test]
    async fn ch15_verifier_collector_setup_counts_empty_password_as_failure() {
        let app = verifier_app().await;
        let cookie = testdb::web_cookie(&app, PASSWORD).await;
        for _ in 0..9 {
            let (status, _, _) = setup(&app, &cookie, "").await;
            assert_eq!(status, StatusCode::FORBIDDEN);
        }
        let failed = crate::router(app.clone())
            .oneshot(json_post("/session", "wrong-login-0123456789"))
            .await
            .unwrap();
        assert_eq!(failed.status(), StatusCode::UNAUTHORIZED);
        let (status, _, _) = setup(&app, &cookie, PASSWORD).await;
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    }

    /// 資格情報の口の失敗も、ログインの口と同じだけ待たせる（CH15.md 壊してはいけないもの 2。review/code.md R3）。
    #[tokio::test]
    async fn ch15_verifier_collector_setup_failure_is_delayed() {
        let delay = Duration::from_millis(300);
        let app = testdb::verifier_app(PASSWORD, delay).await;
        let cookie = testdb::web_cookie(&app, PASSWORD).await;
        // 空の合言葉は照合（Argon2id）を回さず同じ不一致の枝へ行くので、かかる時間がほぼ遅れだけになる
        let started = Instant::now();
        let (status, _, _) = setup(&app, &cookie, "").await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert!(started.elapsed() >= delay, "{:?}", started.elapsed());
    }

    /// 並んだ 2 つの照合は 1 つずつ通り、後の方は前の失敗の数え上げを見る（印を照合の終わりまで持つ。review/code.md R2）。
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn ch15_verifier_concurrent_checks_see_each_others_failure() {
        let app = verifier_app().await;
        let cookie = testdb::web_cookie(&app, PASSWORD).await;
        for _ in 0..9 {
            let (status, _, _) = setup(&app, &cookie, "wrong-password-0123456789").await;
            assert_eq!(status, StatusCode::FORBIDDEN);
        }
        let ((a, _, _), (b, _, _)) = tokio::join!(
            setup(&app, &cookie, "wrong-password-0123456789"),
            setup(&app, &cookie, "wrong-password-9876543210"),
        );
        let mut got = [a, b];
        got.sort();
        assert_eq!(
            got,
            [StatusCode::FORBIDDEN, StatusCode::TOO_MANY_REQUESTS],
            "10 回目の失敗の後に照合を始めた方は 429"
        );
    }

    /// 照らす形の照合は blocking の側で行い、async の worker を塞がない（review/code.md R2 の変異 D2）。
    /// 1 回 poll して Pending なら、照合を待つ間に worker を返している（その場で Argon2id を回すと Ready になる）。
    #[tokio::test]
    async fn ch15_verifier_check_runs_off_the_async_worker() {
        let app = verifier_app().await;
        let body = serde_json::json!({ "password": "wrong-password-0123456789" }).to_string();
        let mut check = std::pin::pin!(web_session::check_given_password(&app, body.as_bytes()));
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(std::future::Future::poll(check.as_mut(), &mut cx).is_pending());
        assert_eq!(check.await, web_session::PasswordCheck::Mismatch);
    }

    /// 取り消された求めの照合も、行を残してから終わる（data-sensitivity「1 件ごとに 1 行」。review/code.md R1）。
    #[tokio::test]
    async fn ch15_verifier_cancelled_requests_still_leave_a_row() {
        let recorder = Recorder::default();
        let app = verifier_app().await.with_access_sink(recorder.clone());
        let cookie = testdb::web_cookie(&app, PASSWORD).await;
        let cases = [
            (
                setup_request(&cookie, "wrong-password-0123456789"),
                "/collector-setup",
                "forbidden",
            ),
            (
                json_post("/session", "wrong-login-0123456789"),
                "/session",
                "login_failed",
            ),
        ];
        // 照合の途中で取り消す。試験が 1 つずつの印を握るので、照合は印を待つ所で止まり、取り消すまで終わらない
        // （review/code.md R10: Argon2id の測定値で時刻を決めると、速いビルドでは照合の前の段で取り消して偽の赤になる）。
        // 取り消すまでの 1 秒は、照合の前の段（JSON とログインの印の確かめ。debug で約 30ms）が終わるのを待つ余裕
        for (req, route, outcome) in cases {
            let held = web_session::hold_login_check_for_test().await;
            let cancelled = tokio::time::timeout(
                Duration::from_secs(1),
                crate::router(app.clone()).oneshot(req),
            )
            .await;
            assert!(cancelled.is_err(), "{route}: 照合の途中で取り消せていない");
            assert_eq!(
                recorder.count(route, outcome),
                0,
                "{route}: 印を握る間は照合が終わらない"
            );
            drop(held);
            let waited = Instant::now();
            while recorder.count(route, outcome) == 0 && waited.elapsed() < Duration::from_secs(5) {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            assert_eq!(recorder.count(route, outcome), 1, "{route}");
        }
    }
}
