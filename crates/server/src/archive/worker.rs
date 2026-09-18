// SPDX-License-Identifier: AGPL-3.0-only
//! 走査と分けた、順番を保つ単一の書庫読み手（ST12 / D1）。

use super::scan::ScanCandidate;

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
