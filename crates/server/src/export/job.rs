// SPDX-License-Identifier: AGPL-3.0-only
//! 書き出しの背景の仕事（ST33 / design D2 / D4 / D10 / D16）。
//!
//! `POST /export` が `spawn` で 1 本だけ走らせる。順は「台帳の始まりの行 → 遅着の印 → 写し（`run_named`）→
//! 台帳の終わりの行 → 前のファイルを消す」。どの段で落ちても始まりと終わりの 2 行が残る（`halt` で止めたときを除く）。
//! 合言葉はこの仕事の引数にだけ渡り、`Running` にも台帳にも置かない。

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::ledger::{self, Done, LedgerRow, RunningView};
use super::{
    free_name, name_stamp, run_named, stage, ExportConfig, ExportError, FailKind, Passphrase,
    Progress, Stage,
};

/// 走っている書き出し。進み具合は本体が更新する。
#[derive(Debug, Clone)]
struct Running {
    export_id: Uuid,
    started_at: DateTime<Utc>,
    progress: Arc<Progress>,
}

/// 走っている書き出し（同時に 1 本。D2）。`App` が持つ。
#[derive(Debug, Clone, Default)]
pub struct ExportJobs(Arc<Mutex<Option<Running>>>);

impl ExportJobs {
    fn slot(&self) -> MutexGuard<'_, Option<Running>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// 走っているものがあれば、その進み具合。
    pub fn running_view(&self) -> Option<RunningView> {
        use std::sync::atomic::Ordering::Relaxed;
        self.slot().as_ref().map(|r| RunningView {
            export_id: r.export_id,
            started_at: r.started_at,
            written: r.progress.written.load(Relaxed),
            total: r.progress.total.load(Relaxed),
        })
    }

    /// 走っているものが無ければ置く。置けなければ `None`（2 本目は 409）。
    fn try_start(&self, cfg: &ExportConfig) -> Option<(Uuid, ExportConfig)> {
        let mut slot = self.slot();
        if slot.is_some() {
            return None;
        }
        let export_id = Uuid::new_v4();
        let progress = Arc::new(Progress::default());
        *slot = Some(Running {
            export_id,
            started_at: Utc::now(),
            progress: progress.clone(),
        });
        Some((
            export_id,
            ExportConfig {
                progress,
                ..cfg.clone()
            },
        ))
    }
}

/// 仕事が終わる（成否・異常終了を問わず）と置いたものを空に戻す。
struct ClearOnDrop(ExportJobs);

impl Drop for ClearOnDrop {
    fn drop(&mut self) {
        *self.0.slot() = None;
    }
}

/// 書き出しを始める。走っているものがあれば `None`。合言葉はこの仕事の中でだけ持つ。
pub fn spawn(
    pool: sqlx::PgPool,
    jobs: &ExportJobs,
    cfg: &ExportConfig,
    passphrase: Passphrase,
) -> Option<Uuid> {
    let (export_id, cfg) = jobs.try_start(cfg)?;
    let guard = ClearOnDrop(jobs.clone());
    tokio::spawn(async move {
        let _guard = guard;
        execute(&pool, &cfg, passphrase, export_id).await;
    });
    Some(export_id)
}

/// 台帳つきの書き出し 1 回。失敗は台帳の終わりの行に残し、ここでは返さない。
pub async fn execute(
    pool: &sqlx::PgPool,
    cfg: &ExportConfig,
    passphrase: Passphrase,
    export_id: Uuid,
) {
    let stamp = name_stamp();
    let planned = cfg.dir.as_deref().map_or_else(String::new, |d| {
        free_name(d, &stamp).to_string_lossy().into_owned()
    });
    if let Err(e) = ledger::insert_start(pool, export_id, &planned, Utc::now()).await {
        tracing::error!(kind = e.kind.as_str(), "台帳に始まりを書けない");
        return;
    }
    let outcome = async {
        stage(cfg, Stage::LateMark, late_mark(pool)).await?;
        run_named(pool, cfg, passphrase, export_id, &stamp).await
    }
    .await;
    match outcome {
        Ok(w) => finish_done(pool, cfg, export_id, w).await,
        Err(e) => {
            tracing::warn!(kind = e.kind.as_str(), "書き出しに失敗した");
            if let Err(e2) = ledger::insert_failed(pool, export_id, &planned, &e.to_string()).await
            {
                tracing::error!(kind = e2.kind.as_str(), "台帳に失敗を書けない");
            }
        }
    }
}

/// 本人が消した滞在のある利用者ごとに、遅れて届いた位置の印を付ける（D6）。
async fn late_mark(pool: &sqlx::PgPool) -> Result<(), ExportError> {
    let fail = |e: &sqlx::Error| {
        let code = e
            .as_database_error()
            .and_then(|d| d.code())
            .map_or_else(|| "unknown".to_string(), |c| c.into_owned());
        ExportError::new(
            FailKind::LateMark,
            format!("遅れて届いた位置の印を付けられない: sqlstate {code}"),
        )
    };
    let users = crate::stay_store::users_with_erased_stays(pool)
        .await
        .map_err(|e| fail(&e))?;
    for user in users {
        crate::stay_store::mark_late_for_user(pool, user)
            .await
            .map_err(|e| fail(&e))?;
    }
    Ok(())
}

async fn finish_done(pool: &sqlx::PgPool, cfg: &ExportConfig, export_id: Uuid, w: super::Written) {
    let file = w.file.clone();
    let measured = tokio::task::spawn_blocking(move || {
        let bytes = std::fs::metadata(&file)?.len();
        ledger::sha256_of(&file).map(|d| (bytes, d))
    })
    .await;
    let (bytes, digest) = match measured {
        Ok(Ok(v)) => v,
        _ => {
            tracing::error!(kind = "io", "書いたファイルの大きさとハッシュを取れない");
            let reason = format!("{}: 書いたファイルを読めない", FailKind::Io.as_str());
            let _ =
                ledger::insert_failed(pool, export_id, &w.file.to_string_lossy(), &reason).await;
            return;
        }
    };
    let rows: serde_json::Map<String, serde_json::Value> = w
        .dumped
        .files
        .iter()
        .map(|f| (f.name.to_string(), serde_json::Value::from(f.rows)))
        .collect();
    let file = w.file.to_string_lossy().into_owned();
    let done = Done {
        file: &file,
        snapshot_at: w.dumped.snapshot_at,
        rows: serde_json::Value::Object(rows),
        bytes: i64::try_from(bytes).unwrap_or(i64::MAX),
        written: &w.dumped.written,
        digest: &digest,
    };
    // 台帳の終わりの行を書く前にサーバが止まった状態（試験）。何も書かずに打ち切る。
    if (cfg.hooks.halt)(Stage::Finish) {
        return;
    }
    let wrote = stage(
        cfg,
        Stage::Finish,
        ledger::insert_done(pool, export_id, &done),
    )
    .await;
    if let Err(e) = wrote {
        tracing::error!(kind = e.kind.as_str(), "台帳に終わりを書けない");
        let reason = e.to_string();
        let _ = ledger::insert_failed(pool, export_id, &file, &reason).await;
        return;
    }
    // 終わりの行を書いた後に前のファイルを消す（消している間に止まっても、書き終えたことは台帳に残る。D16）。
    let _ = stage(cfg, Stage::DeletePrevious, delete_previous(pool, export_id)).await;
}

/// 台帳で `done` の他の書き出しのファイルのうち、書いたその物のままのものを消す。消せなくても続ける
/// （残ったものは状態の `leftover` に出て、次の書き出しがもう一度消す）。
async fn delete_previous(pool: &sqlx::PgPool, current: Uuid) -> Result<(), ExportError> {
    let rows = ledger::read_ledger(pool).await?;
    let _ = tokio::task::spawn_blocking(move || {
        for r in ledger::removable_previous(&rows, Some(current)) {
            if std::fs::remove_file(&r.file).is_err() {
                tracing::warn!(kind = "io", "前の書き出しのファイルを消せない");
            }
        }
    })
    .await;
    Ok(())
}

/// 状態を読む（D11）。台帳と置き場とメモリから作る。
pub async fn read_status(
    pool: &sqlx::PgPool,
    jobs: &ExportJobs,
) -> Result<ledger::Status, ExportError> {
    let running = jobs.running_view();
    let rows = ledger::read_ledger(pool).await?;
    let derived = ledger::derive(&rows, running.as_ref().map(|r| r.export_id));
    let erased = match &derived.last {
        Some(l) => ledger::erased_after(pool, l.export_id).await?,
        None => 0,
    };
    tokio::task::spawn_blocking(move || {
        derived.into_status(
            running,
            erased,
            |f| std::path::Path::new(f).is_file(),
            |r: &LedgerRow| ledger::is_untouched(r),
        )
    })
    .await
    .map_err(|_| ExportError::new(FailKind::Io, "状態を作れない"))
}
