// SPDX-License-Identifier: AGPL-3.0-only
//! 走査と分けた、順番を保つ単一の書庫読み手（ST12 / D1）。

use super::scan::ScanCandidate;

/// 解析前に、書庫を開いて既知・未読・読めない中身を数える結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Inspection {
    pub known: usize,
    pub skipped: usize,
    pub unreadable: usize,
}

/// 読み手が処理する候補を、値をログへ出さずに検査する。
pub fn inspect(candidate: ScanCandidate) -> Result<Inspection, super::open::OpenError> {
    let files = super::open::open_archive(&candidate.path)?;
    let classified = super::classify::classify_files(&files);
    Ok(Inspection {
        known: classified.known.len(),
        skipped: classified.skipped,
        unreadable: classified.unreadable,
    })
}

/// 送られた順に 1 冊ずつ読み終える。走査側は別taskでこの送信側を保持する。
pub async fn read_in_order<F, Fut>(
    mut receiver: tokio::sync::mpsc::Receiver<ScanCandidate>,
    mut read: F,
) where
    F: FnMut(ScanCandidate) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    while let Some(candidate) = receiver.recv().await {
        read(candidate).await;
    }
}

/// 走査と直列読み手を背景で起こす。利用者が未設定なら呼び出し側は起こさない。
pub fn spawn_inspecting(
    pool: sqlx::PgPool,
    config: super::config::ArchiveConfig,
    user_id: uuid::Uuid,
) {
    let (sender, receiver) = tokio::sync::mpsc::channel(32);
    let scan_pool = pool.clone();
    tokio::spawn(async move {
        loop {
            match super::scan::scan_once(&scan_pool, &config, user_id).await {
                Ok(candidates) => {
                    for candidate in candidates {
                        if sender.send(candidate).await.is_err() {
                            return;
                        }
                    }
                }
                Err(error) => {
                    tracing::warn!(kind = "archive_scan", error = %error, "書庫の置き場を走査できない")
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(config.scan_sec)).await;
        }
    });
    tokio::spawn(read_in_order(receiver, move |candidate| {
        let pool = pool.clone();
        async move {
            let sha256 = candidate.sha256.clone();
            let inbox_kind = if candidate.from_downloads {
                "downloads"
            } else {
                "inbox"
            };
            match inspect(candidate) {
            Ok(result) => {
                if let Err(error) = sqlx::query(
                    "INSERT INTO core.archive_ledger
                       (user_id, sha256, parser_version, outcome, inbox_kind, unreadable_count, skipped_file_count)
                     VALUES ($1, $2, $3, 'read', $4, $5, $6)
                     ON CONFLICT DO NOTHING",
                )
                .bind(user_id)
                .bind(sha256)
                .bind(super::PARSER_VERSION)
                .bind(inbox_kind)
                .bind(i32::try_from(result.unreadable).unwrap_or(i32::MAX))
                .bind(i32::try_from(result.skipped).unwrap_or(i32::MAX))
                .execute(&pool)
                .await
                {
                    tracing::warn!(kind = "archive_ledger", error = %error, "書庫の台帳を残せない");
                } else {
                    tracing::info!(
                        kind = "archive_inspect",
                        known = result.known,
                        skipped = result.skipped,
                        unreadable = result.unreadable,
                        "書庫を検査した"
                    );
                }
            }
            Err(error) => tracing::warn!(kind = error.kind(), "書庫を開けない"),
        }
        }
    }));
}
