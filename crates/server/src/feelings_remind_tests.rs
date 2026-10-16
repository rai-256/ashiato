// SPDX-License-Identifier: AGPL-3.0-only
//! 通知の時刻の台帳と通知の判断の口（ST17 / Task 5。design D9）。**利用者で隔離する**（`testdb::user()`）。
#![allow(clippy::unwrap_used)]

use super::*;
use crate::feelings_ingest_tests::{app, auth, valence, write_day};
use crate::testdb;

async fn set(app: &App, user: uuid::Uuid, at: &str) -> Result<(), (StatusCode, String)> {
    feelings_reminder_post(
        State(app.clone()),
        auth(),
        Json(ReminderSetRequest {
            user_id: Some(user),
            at: at.to_string(),
        }),
    )
    .await
    .map(|_| ())
}

async fn decide(app: &App, user: uuid::Uuid, date: &str) -> serde_json::Value {
    let Json(d) = feelings_reminder_get(
        State(app.clone()),
        auth(),
        Query(ReminderQuery {
            date: date.to_string(),
            user_id: Some(user),
        }),
    )
    .await
    .unwrap();
    serde_json::to_value(d).unwrap()
}

async fn ledger_rows(app: &App, user: uuid::Uuid) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM core.feeling_reminder WHERE user_id = $1")
        .bind(user)
        .fetch_one(&app.pool)
        .await
        .unwrap()
}

#[test]
fn st17_remind_default_is_pinned_to_2100() {
    assert_eq!(feelings::DEFAULT_REMINDER_AT, "21:00");
}

// Scenario: 通知の時刻の既定は 21 時
#[tokio::test]
async fn st17_remind_default_is_2100() {
    let app = app().await;
    let user = testdb::user();
    assert_eq!(decide(&app, user, "2026-10-07").await["at"], "21:00");
}

// Scenario: 通知の時刻を変えると次の判断から効く
#[tokio::test]
async fn st17_remind_change_takes_effect_on_next_decision() {
    let app = app().await;
    let user = testdb::user();
    set(&app, user, "22:30").await.unwrap();
    assert_eq!(decide(&app, user, "2026-10-07").await["at"], "22:30");
}

// Scenario: 通知の時刻の変更は台帳に積まれる
#[tokio::test]
async fn st17_remind_changes_pile_up_in_the_ledger() {
    let app = app().await;
    let user = testdb::user();
    set(&app, user, "22:30").await.unwrap();
    set(&app, user, "20:00").await.unwrap();
    assert_eq!(ledger_rows(&app, user).await, 2);
    assert_eq!(decide(&app, user, "2026-10-07").await["at"], "20:00");
}

// Scenario: 書いた日の判断は記録ありになる
#[tokio::test]
async fn st17_remind_written_day_is_recorded() {
    let app = app().await;
    let user = testdb::user();
    write_day(
        &app,
        user,
        "2026-10-07",
        "2026-10-07T21:30:00+09:00",
        valence(1),
        None,
    )
    .await;
    assert_eq!(decide(&app, user, "2026-10-07").await["recorded"], true);
}

// Scenario: 書いていない日の判断は記録なしになる
#[tokio::test]
async fn st17_remind_unwritten_day_is_not_recorded() {
    let app = app().await;
    let user = testdb::user();
    write_day(
        &app,
        user,
        "2026-10-06",
        "2026-10-06T21:30:00+09:00",
        valence(1),
        None,
    )
    .await;
    assert_eq!(decide(&app, user, "2026-10-07").await["recorded"], false);
}

// Scenario: 通知の判断は主観の中身を返さない
#[tokio::test]
async fn st17_remind_decision_has_only_two_keys_and_no_content() {
    let app = app().await;
    let user = testdb::user();
    write_day(
        &app,
        user,
        "2026-10-07",
        "2026-10-07T21:30:00+09:00",
        valence(2),
        Some("ひみつのひとこと"),
    )
    .await;
    let v = decide(&app, user, "2026-10-07").await;
    let mut keys: Vec<_> = v.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(keys, ["at", "recorded"]);
    let body = v.to_string();
    assert!(
        !body.contains("ひみつ") && !body.contains("valence"),
        "{body}"
    );
}

// Scenario: 時刻でない通知の時刻は断られる
#[tokio::test]
async fn st17_remind_rejects_non_time_and_keeps_current() {
    let app = app().await;
    let user = testdb::user();
    set(&app, user, "22:30").await.unwrap();
    for bad in [
        "24:10",
        "9:05",
        "21:60",
        "21:00:00",
        "ab:cd",
        "",
        "２１:００",
    ] {
        let e = set(&app, user, bad).await.unwrap_err();
        assert_eq!(e.0, StatusCode::BAD_REQUEST, "{bad}");
    }
    assert_eq!(ledger_rows(&app, user).await, 1);
    assert_eq!(decide(&app, user, "2026-10-07").await["at"], "22:30");
}
