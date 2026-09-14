// SPDX-License-Identifier: AGPL-3.0-only
//! 収集開始日（第 8 回 Q29）と、その修復。**本物の PostgreSQL に対して**確かめる。
#![allow(clippy::unwrap_used)]

use super::*;
use crate::testdb;

//                                              端末の時計が狂った信号と収集開始日
/// 登録簿に行ができた日より前の時刻は、収集開始日の計算から外れる（tasks 12.2）。
///
/// **記録も信号も捨てない。** 外すのは収集開始日への寄与だけ —— 本人の答えは
/// 「受けるが、収集開始日の計算から外す」（第 8 回 Q29）。
///
/// Scenario: 登録より前の時刻の信号は開始日を動かさない
#[tokio::test]
async fn clock_skew_does_not_move_started_on() {
    let pool = testdb::pool().await;
    let name = testdb::source_registered_on(&pool, "skew", SIX_HOURS, "2026-04-01").await;
    let u = testdb::user();

    // 端末の時計が 27 年戻った信号。**受け取って保存する**
    testdb::put_heartbeat(&pool, u, &name, "1999-01-01T03:00:00Z", true).await;
    touch_started_on(
        &pool,
        &name,
        "1999-01-01T03:00:00Z".parse().unwrap(),
        Arrival::Heartbeat,
    )
    .await
    .unwrap();

    let got = sources(&pool, std::slice::from_ref(&name)).await.unwrap();
    assert_eq!(
        got[0].collection_started_on, None,
        "登録より前の信号が収集開始日を動かした（1999 年に落ちると前にしか動かないので戻せない）"
    );

    // **信号そのものは残っている**（捨てていないことを、読み出し口から確かめる）
    let d = testdb::date("1999-01-01");
    let cov = of_source(&pool, Some(u), &got[0], d, d).await.unwrap();
    assert_eq!(
        cov.days[0].intervals.len(),
        1,
        "保存したはずの生存信号が読み出せない（外すのは開始日の計算だけ）"
    );

    // 登録より後の信号は、これまでどおり開始日を動かす
    testdb::put_heartbeat(&pool, u, &name, "2026-04-05T03:00:00Z", true).await;
    touch_started_on(
        &pool,
        &name,
        "2026-04-05T03:00:00Z".parse().unwrap(),
        Arrival::Heartbeat,
    )
    .await
    .unwrap();
    let got = sources(&pool, std::slice::from_ref(&name)).await.unwrap();
    assert_eq!(
        got[0].collection_started_on,
        Some(testdb::date("2026-04-05")),
        "登録より後の信号まで外れている"
    );
}

/// 既に汚れている収集開始日を、移行 0007 が引き直す（tasks 12.1）。
///
/// **受け口に条件を足すだけでは足りない。** 収集開始日は前にしか動かないので、
/// 一度 1999 年に落ちた行は正しい日を送り直しても戻らない（第 8 回 Q29 の論点そのもの）。
///
/// Scenario: 汚れた収集開始日は引き直せる
#[tokio::test]
async fn migration_repairs_polluted_started_on() {
    let pool = testdb::pool().await;
    let u = testdb::user();

    // (a) 登録より後の記録があるソース —— 引き直すとその日に戻る。
    // **汚れ方まで本物にする** —— 1999 年の開始日は、1999 年に発信されたと称する
    // 生存信号が書いたもの。信号を置かずに開始日だけ 1999 にするのは、
    // **実際には起こりえない状態**（そこから直せても、直したことにならない）
    let good = testdb::source_registered_on(&pool, "repair-a", SIX_HOURS, "2026-04-01").await;
    testdb::put_heartbeat(&pool, u, &good, "1999-01-01T03:00:00Z", true).await;
    testdb::put_event(&pool, u, &good, "2026-04-07T12:00:00+09:00").await;
    testdb::set_started_on(&pool, &good, "1999-01-01").await;

    // (b) 登録より前の信号しか無いソース —— 引き直すと「まだ開始していない」に戻る
    let bad = testdb::source_registered_on(&pool, "repair-b", SIX_HOURS, "2026-04-01").await;
    testdb::put_heartbeat(&pool, u, &bad, "1999-01-01T03:00:00Z", true).await;
    testdb::set_started_on(&pool, &bad, "1999-01-01").await;

    // 移行を当て直す（`run()` は起動のたびに全部の版を当てる）。
    //
    // **トランザクションの中で当てて、読んでから巻き戻す。** 0007 は登録簿の全行を
    // 引き直すので、そのまま流すと**並んで走っている他のテストの収集開始日まで書き換える**
    // （実測: 達成日数の検査が分母 0 で落ちた）。1 つの DB を共有する足場なので、
    // 「全行に効く移行」を試すテストだけは自分の外へ出さない。
    let mut tx = pool.begin().await.unwrap();
    sqlx::raw_sql(repair_sql()).execute(&mut *tx).await.unwrap();
    let got: Vec<(String, Option<chrono::NaiveDate>)> = sqlx::query_as(
        "SELECT logical_source, collection_started_on FROM core.source
          WHERE logical_source = ANY($1)",
    )
    .bind(vec![good.clone(), bad.clone()])
    .fetch_all(&mut *tx)
    .await
    .unwrap();
    tx.rollback().await.unwrap();

    let by = |n: &str| got.iter().find(|r| r.0 == n).unwrap().1;
    assert_eq!(
        by(&good),
        Some(testdb::date("2026-04-07")),
        "登録以降のいちばん古い記録の日に引き直されていない"
    );
    assert_eq!(
        by(&bad),
        None,
        "登録より前の信号しか無いソースが「開始済み」のまま残っている"
    );
}

/// **登録日ちょうど**の記録は収集開始日を作る（同 R7 / G2）。
///
/// 閾値が `>=` から `>` に倒れると、**登録したその日に初回の記録が来るソース
/// （＝通常の導入手順そのもの）が永久に「未開始」で固まる** —— Q29 が直そうとした型と同じ。
///
/// Scenario: 登録より前の時刻の信号は開始日を動かさない
#[tokio::test]
async fn a_record_on_the_registration_day_starts_collection() {
    let pool = testdb::pool().await;
    let name = testdb::source_registered_on(&pool, "edge", SIX_HOURS, "2026-04-01").await;

    // 登録日の JST 0 時ちょうど（その日でいちばん早い時刻）
    touch_started_on(
        &pool,
        &name,
        "2026-03-31T15:00:00Z".parse().unwrap(),
        Arrival::Heartbeat,
    )
    .await
    .unwrap();
    let got = sources(&pool, std::slice::from_ref(&name)).await.unwrap();
    assert_eq!(
        got[0].collection_started_on,
        Some(testdb::date("2026-04-01")),
        "登録日ちょうどの記録が弾かれている（閾値が > に倒れている）"
    );

    // その 1 秒前（前日の 23:59:59 JST）は弾かれる
    let other = testdb::source_registered_on(&pool, "edge2", SIX_HOURS, "2026-04-01").await;
    touch_started_on(
        &pool,
        &other,
        "2026-03-31T14:59:59Z".parse().unwrap(),
        Arrival::Heartbeat,
    )
    .await
    .unwrap();
    let got = sources(&pool, std::slice::from_ref(&other)).await.unwrap();
    assert_eq!(
        got[0].collection_started_on, None,
        "登録日の前日が通っている"
    );
}

/// 引き直しは**何度当てても同じ**で、**正規の収集開始日を後ろへ動かさない**（同 C-3 / G4 / M-3）。
///
/// `migrate()` は版管理表を持たず**起動のたびに当て直す**ので、条件を付けないと
/// 「記録を破棄した後に再起動する」だけで⑤「破棄された期間」が⑦「導入前」に化ける。
///
/// Scenario: 汚れた収集開始日は引き直せる
#[tokio::test]
async fn repair_is_idempotent_and_never_moves_a_clean_date_forward() {
    let pool = testdb::pool().await;
    let name = testdb::source_registered_on(&pool, "idem", SIX_HOURS, "2026-04-01").await;
    let u = testdb::user();
    // 正規の収集開始日。**記録はもう残っていない**（破棄・保持期間の刈り取りの後の形）
    testdb::set_started_on(&pool, &name, "2026-04-05").await;
    testdb::put_event(&pool, u, &name, "2026-04-20T12:00:00+09:00").await;

    const READ: &str = "SELECT collection_started_on FROM core.source WHERE logical_source = $1";
    let mut tx = pool.begin().await.unwrap();
    sqlx::raw_sql(repair_sql()).execute(&mut *tx).await.unwrap();
    let once: (Option<chrono::NaiveDate>,) = sqlx::query_as(READ)
        .bind(&name)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    sqlx::raw_sql(repair_sql()).execute(&mut *tx).await.unwrap();
    let twice: (Option<chrono::NaiveDate>,) = sqlx::query_as(READ)
        .bind(&name)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    let (once, twice) = (once.0, twice.0);

    assert_eq!(
        once,
        Some(testdb::date("2026-04-05")),
        "汚れていない収集開始日が後ろへ動いた（窓が縮み、破棄した期間が⑦に化ける）"
    );
    assert_eq!(twice, once, "2 度当てると結果が変わる");
}

/// **記録には閾値が掛からない**（第 9 回 Q32）。
///
/// 第 8 回 Q29 は記録にも閾値を掛けていた。`registered_at` の既定が `now()` なので、
/// **登録簿に行を足してから過去のデータを流し込む運用**（端末にある写真・
/// 導入時点で取れるブラウザ履歴・Takeout 系）では、記録が何万件あっても
/// 収集開始日が作れず、**全日が⑦「導入前」**になった（実測）。
///
/// Scenario: 登録より前の時刻の記録は開始日を動かす
#[tokio::test]
async fn an_old_record_starts_collection_even_before_registration() {
    let pool = testdb::pool().await;
    let name = testdb::source_registered_on(&pool, "backfill", SIX_HOURS, "2026-04-01").await;
    let u = testdb::user();

    // 登録より 2 年前に撮られた写真を取り込む
    testdb::put_event(&pool, u, &name, "2024-05-20T12:00:00+09:00").await;
    touch_started_on(
        &pool,
        &name,
        "2024-05-20T03:00:00Z".parse().unwrap(),
        Arrival::Record,
    )
    .await
    .unwrap();

    let got = sources(&pool, std::slice::from_ref(&name)).await.unwrap();
    assert_eq!(
        got[0].collection_started_on,
        Some(testdb::date("2024-05-20")),
        "登録より前の記録が収集開始日を作れていない（過去ぶんの取り込みが全日⑦になる）"
    );

    // **その日はもう⑦ではない**（記録があるので①）
    assert_eq!(
        state_on(&pool, u, &got[0], "2024-05-20").await,
        DayState::Recorded,
        "記録のある日が「導入前」のまま"
    );

    // **同じ日の生存信号は、いまも閾値で外れる**（Q29 は変わっていない）
    let beat = testdb::source_registered_on(&pool, "backfill-hb", SIX_HOURS, "2026-04-01").await;
    testdb::put_heartbeat(&pool, u, &beat, "2024-05-20T03:00:00Z", true).await;
    touch_started_on(
        &pool,
        &beat,
        "2024-05-20T03:00:00Z".parse().unwrap(),
        Arrival::Heartbeat,
    )
    .await
    .unwrap();
    let got = sources(&pool, std::slice::from_ref(&beat)).await.unwrap();
    assert_eq!(
        got[0].collection_started_on, None,
        "生存信号にまで閾値が効かなくなっている（Q29 が塞いだ穴が開く）"
    );
}

/// 引き直しは、**記録に支えられた古い開始日を後ろへ動かさない**（第 9 回 Q32）。
///
/// 「汚れている」の判定を「登録より前」だけにしていると、過去ぶんを流し込んだソースの
/// **正しい**開始日を毎起動で消してしまう。
///
/// Scenario: 汚れた収集開始日は引き直せる
#[tokio::test]
async fn repair_keeps_a_backfilled_start_date() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    // 過去ぶんを流し込んだソース。開始日は登録より前だが**記録が支えている**
    let back = testdb::source_registered_on(&pool, "rep-back", SIX_HOURS, "2026-04-01").await;
    testdb::put_event(&pool, u, &back, "2024-05-20T12:00:00+09:00").await;
    testdb::set_started_on(&pool, &back, "2024-05-20").await;
    // 時計の狂った信号で汚れたソース。**支える記録が無い**
    let bad = testdb::source_registered_on(&pool, "rep-bad", SIX_HOURS, "2026-04-01").await;
    testdb::put_heartbeat(&pool, u, &bad, "1999-01-01T03:00:00Z", true).await;
    testdb::put_event(&pool, u, &bad, "2026-04-07T12:00:00+09:00").await;
    testdb::set_started_on(&pool, &bad, "1999-01-01").await;

    let mut tx = pool.begin().await.unwrap();
    sqlx::raw_sql(repair_sql()).execute(&mut *tx).await.unwrap();
    let got: Vec<(String, Option<chrono::NaiveDate>)> = sqlx::query_as(
        "SELECT logical_source, collection_started_on FROM core.source
          WHERE logical_source = ANY($1)",
    )
    .bind(vec![back.clone(), bad.clone()])
    .fetch_all(&mut *tx)
    .await
    .unwrap();
    tx.rollback().await.unwrap();
    let by = |n: &str| got.iter().find(|r| r.0 == n).unwrap().1;

    assert_eq!(
        by(&back),
        Some(testdb::date("2024-05-20")),
        "記録に支えられた開始日が後ろへ動いた（過去ぶんの取り込みが毎起動で消える）"
    );
    assert_eq!(
        by(&bad),
        Some(testdb::date("2026-04-07")),
        "支える記録の無い開始日が引き直されていない"
    );
}

/// 引き直しは、**記録を破棄した後でも開始日を前へ動かさない**（C-3 / 第 9 回 Q32）。
///
/// 「支える記録が無い」だけを汚れの印にすると、⑤「破棄された期間」を作った次の再起動で
/// 開始日がその期間の後ろへ動き、**破棄が⑦「導入前」に化ける**（`coverage_span` の行だけが残る）。
/// 汚れの印は「**閾値より前の生存信号の日ちょうど**」まで狭めてある。
///
/// Scenario: 汚れた収集開始日は引き直せる
#[tokio::test]
async fn repair_does_not_move_forward_after_records_are_dropped() {
    let pool = testdb::pool().await;
    let u = testdb::user();
    let name = testdb::source_registered_on(&pool, "dropped", SIX_HOURS, "2026-04-01").await;
    // 過去ぶんを流し込んだソース。開始日は 2024-05-20
    testdb::set_started_on(&pool, &name, "2024-05-20").await;
    // **その時代の記録はもう無い**（破棄した）。残っているのは後の記録だけ
    testdb::put_event(&pool, u, &name, "2026-04-20T12:00:00+09:00").await;
    testdb::put_span(
        &pool,
        u,
        &name,
        "dropped",
        "2024-05-20T00:00:00+09:00",
        Some("2024-06-01T00:00:00+09:00"),
    )
    .await;

    let mut tx = pool.begin().await.unwrap();
    sqlx::raw_sql(repair_sql()).execute(&mut *tx).await.unwrap();
    let got: (Option<chrono::NaiveDate>,) =
        sqlx::query_as("SELECT collection_started_on FROM core.source WHERE logical_source = $1")
            .bind(&name)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    tx.rollback().await.unwrap();

    assert_eq!(
        got.0,
        Some(testdb::date("2024-05-20")),
        "破棄した後の再起動で開始日が前へ動いた（⑤が⑦に化ける）"
    );
}
