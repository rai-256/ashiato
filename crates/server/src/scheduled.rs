// SPDX-License-Identifier: AGPL-3.0-only
//! 定期取得の枠（ST13 / design D5 / D6）。
//!
//! 取り方が「定期取得」の論理ソースごとに task を 1 つ起こし、起動の直後に 1 回取りに行き、終わってから
//! 登録簿の取得の間隔を読み直して眠り、繰り返す。task が 1 つなので同じソースの取得は重ならない。
//! **壁時計を読まない**（時間は `tokio::time` の戻らない時計だけ。前回の時刻も持たない）。
//! `tools/check-scheduled-clock.sh` がこのファイルに壁時計を読む呼び出しが無いことを見る。
//! 生存信号は枠に持たせない（D6）。ログに出すのは種別と論理ソースの名前だけ。

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

pub type FetchFuture<'a> = Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>>;

/// 論理ソース 1 本ぶんの取る手。
pub trait Fetcher: Send + Sync {
    fn logical_source(&self) -> &str;
    fn fetch<'a>(&'a self, pool: &'a sqlx::PgPool, user_id: uuid::Uuid) -> FetchFuture<'a>;
}

/// 本番の取る手の一覧。いまは定期取得するソースが無いので空（後の Story が足す）。
pub fn fetchers() -> Vec<Box<dyn Fetcher>> {
    Vec::new()
}

/// 落とすと定期取得が止まる手。本番は取り込み器と同じく握ったまま持つ。
#[derive(Debug)]
pub struct ScheduledHandle {
    supervisor: tokio::task::JoinHandle<()>,
}

impl ScheduledHandle {
    /// 全てのソースの task が終わった（取る手が 1 つも無かったときも真になる）。
    pub fn is_finished(&self) -> bool {
        self.supervisor.is_finished()
    }
}

impl Drop for ScheduledHandle {
    fn drop(&mut self) {
        // 監督の task が持つ JoinSet も、落ちるときにソースごとの task を止める
        self.supervisor.abort();
    }
}

/// 眠る間隔の下限。登録簿の取得の間隔が空・0・負のとき、0 秒の眠りで取得が間を置かずに
/// 回り続けないよう、ここまで引き上げる（`interval_from`）。間隔の行を読めないときは下限でなく前の値を使う。
const MIN_INTERVAL_SEC: u64 = 1;

/// 起動時に登録簿の定期取得の行を読めなかったときに、読み直すまでの待ち（final review R4）。
/// 1 回の失敗で諦めていたときは、DB が起動より遅れて立ち上がるだけで再起動まで取りに行かなかった。
const REGISTRY_RETRY_SEC: u64 = 60;

#[must_use = "落とすと定期取得が止まる。本番は握ったまま持つ"]
pub fn spawn(pool: sqlx::PgPool, user_id: uuid::Uuid) -> ScheduledHandle {
    spawn_with(fetchers(), pool, user_id)
}

/// 取る手を差し込める入口（試験用。本番は `spawn`）。
#[must_use = "落とすと定期取得が止まる"]
pub fn spawn_with(
    fetchers: Vec<Box<dyn Fetcher>>,
    pool: sqlx::PgPool,
    user_id: uuid::Uuid,
) -> ScheduledHandle {
    let supervisor = tokio::spawn(async move {
        // 読めるまで読み直す。warn は読めなくなった 1 回だけ（ログは種別だけ。本文は出さない）。
        let mut warned = false;
        let rows = loop {
            match scheduled_rows(&pool).await {
                Ok(rows) => break rows,
                Err(_) => {
                    if !std::mem::replace(&mut warned, true) {
                        tracing::warn!(
                            kind = "scheduled_registry",
                            "定期取得の行を読めない。読めるまで読み直す"
                        );
                    }
                    tokio::time::sleep(Duration::from_secs(REGISTRY_RETRY_SEC)).await;
                }
            }
        };
        if warned {
            tracing::info!(kind = "scheduled_registry", "定期取得の行を読めた");
        }
        let mut fetchers: Vec<Option<std::sync::Arc<dyn Fetcher>>> = fetchers
            .into_iter()
            .map(|f| Some(std::sync::Arc::from(f)))
            .collect();
        let mut tasks = tokio::task::JoinSet::new();
        let mut names = std::collections::HashMap::new();
        for (logical_source, interval_sec) in rows {
            let found = fetchers.iter_mut().find(|f| {
                f.as_ref()
                    .is_some_and(|f| f.logical_source() == logical_source)
            });
            let Some(fetcher) = found.and_then(Option::take) else {
                tracing::warn!(
                    kind = "scheduled_no_fetcher",
                    logical_source,
                    "定期取得の取る手が無いので取りに行かない"
                );
                continue;
            };
            let task = tasks.spawn(run_source(pool.clone(), user_id, fetcher, interval_sec));
            names.insert(task.id(), logical_source);
        }
        // ソースの task は終わらない。終わったなら取る手が panic した（か止められた）ので、
        // 黙って捨てずに種別と論理ソースの名前を残す（final review R4）。
        while let Some(joined) = tasks.join_next_with_id().await {
            if let Err(error) = joined {
                let logical_source = names.get(&error.id()).map_or("", String::as_str);
                tracing::warn!(
                    kind = "scheduled_task_ended",
                    logical_source,
                    failure = if error.is_panic() {
                        "panic"
                    } else {
                        "cancelled"
                    },
                    "定期取得の task が止まった"
                );
            }
        }
    });
    ScheduledHandle { supervisor }
}

async fn scheduled_rows(pool: &sqlx::PgPool) -> anyhow::Result<Vec<(String, u64)>> {
    let rows: Vec<(String, Option<i32>)> = sqlx::query_as(
        "SELECT logical_source, fetch_interval_sec FROM core.source
          WHERE acquisition = 'scheduled' AND fetch_trigger = 'interval'
          ORDER BY logical_source",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(name, sec)| (name, interval_from(sec)))
        .collect())
}

fn interval_from(sec: Option<i32>) -> u64 {
    u64::try_from(sec.unwrap_or(0))
        .unwrap_or(0)
        .max(MIN_INTERVAL_SEC)
}

/// 起動の直後に 1 回 → 終わってから間隔を読み直して眠る → 繰り返す。失敗は次の契機へ進む。
async fn run_source(
    pool: sqlx::PgPool,
    user_id: uuid::Uuid,
    fetcher: std::sync::Arc<dyn Fetcher>,
    mut interval_sec: u64,
) {
    let source = fetcher.logical_source().to_owned();
    loop {
        if let Err(error) = fetcher.fetch(&pool, user_id).await {
            tracing::warn!(
                kind = "scheduled_fetch_failed",
                logical_source = source,
                failure = error_kind(&error),
                "定期取得に失敗した。次の契機で取りに行く"
            );
        }
        match read_interval(&pool, &source).await {
            Ok(sec) => interval_sec = sec,
            Err(_) => {
                tracing::warn!(
                    kind = "scheduled_interval",
                    logical_source = source,
                    "取得の間隔を読めない。前の値を使う"
                );
            }
        }
        tokio::time::sleep(Duration::from_secs(interval_sec)).await;
    }
}

/// 失敗の本文は記録の中身を含みうるので出さない。出すのは種別だけ。
fn error_kind(error: &anyhow::Error) -> &'static str {
    if error.downcast_ref::<sqlx::Error>().is_some() {
        "db"
    } else {
        "fetch"
    }
}

async fn read_interval(pool: &sqlx::PgPool, source: &str) -> anyhow::Result<u64> {
    let sec: Option<i32> = sqlx::query_scalar(
        "SELECT fetch_interval_sec FROM core.source
          WHERE logical_source = $1 AND acquisition = 'scheduled' AND fetch_trigger = 'interval'",
    )
    .bind(source)
    .fetch_optional(pool)
    .await?
    .flatten();
    sec.map(|s| interval_from(Some(s)))
        .ok_or_else(|| anyhow::anyhow!("取得の間隔の行が無い"))
}
