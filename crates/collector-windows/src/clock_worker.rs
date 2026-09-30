// SPDX-License-Identifier: AGPL-3.0-only
//! 基準の読み取りを見回りの輪の外で行う作業スレッド（design D7 / D11）。
//!
//! `/healthz`（最長で要求の打ち切りまで）と `w32tm`（5 秒で打ち切り）は、見回りの輪の中で待つと
//! その間の前景の切り替えを取りこぼす。**1 本の作業スレッドが両方を読み、結果を通り道で返す。**
//! 見回りは毎回 `poll` で覗くだけで、待たない。
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;

use crate::clock::{ReferenceClock, ReferenceReading};
use crate::time_sync::{TimeSyncError, TimeSyncReading, TimeSyncSource};

/// 作業スレッドが返す 1 回分の読み取り。
#[derive(Debug)]
pub struct ClockReading {
    /// 基準の出どころ（`ReferenceClock::source`）。
    pub source: String,
    /// 取り込み口の応答から読んだ基準。
    pub reference: anyhow::Result<ReferenceReading>,
    /// Windows の時刻同期の状態。
    pub time_sync: Result<TimeSyncReading, TimeSyncError>,
}

/// 見回りが通り道を覗いた結果。
#[derive(Debug)]
pub enum WorkerPoll {
    /// まだ読んでいる。
    Pending,
    /// 読み終わった。
    Done(Box<ClockReading>),
    /// 結果を返さずに通り道が閉じた（panic など）。`reason: worker_failed` として扱う。
    Failed,
}

/// 走っている作業スレッド 1 本。**走っている間は次の測定を始めない**（持つ側が守る）。
#[derive(Debug)]
pub struct ClockWorker {
    rx: Receiver<ClockReading>,
    #[cfg(test)]
    settled: Option<WorkerPoll>,
}

impl ClockWorker {
    /// 作業スレッドを起こす。起こせなかったときも通り道は閉じるので `Failed` になる。
    pub fn spawn(reference: Arc<dyn ReferenceClock>, time_sync: Arc<dyn TimeSyncSource>) -> Self {
        let (tx, rx) = mpsc::channel();
        let _ = std::thread::Builder::new()
            .name("clock-skew-reader".into())
            .spawn(move || {
                let reading = ClockReading {
                    source: reference.source(),
                    reference: reference.now(),
                    time_sync: time_sync.read(),
                };
                let _ = tx.send(reading);
            });
        Self {
            rx,
            #[cfg(test)]
            settled: None,
        }
    }

    /// 通り道を覗く。**待たない。**
    pub fn poll(&mut self) -> WorkerPoll {
        #[cfg(test)]
        if let Some(p) = self.settled.take() {
            return p;
        }
        match self.rx.try_recv() {
            Ok(r) => WorkerPoll::Done(Box::new(r)),
            Err(TryRecvError::Empty) => WorkerPoll::Pending,
            Err(TryRecvError::Disconnected) => WorkerPoll::Failed,
        }
    }

    /// 試験用: 作業スレッドが終わるまで待ち、結果を次の `poll` に取っておく。
    #[cfg(test)]
    pub(crate) fn settle(&mut self) {
        use std::sync::mpsc::RecvTimeoutError;
        if self.settled.is_some() {
            return;
        }
        self.settled = match self.rx.recv_timeout(std::time::Duration::from_secs(20)) {
            Ok(r) => Some(WorkerPoll::Done(Box::new(r))),
            Err(RecvTimeoutError::Disconnected) => Some(WorkerPoll::Failed),
            Err(RecvTimeoutError::Timeout) => None,
        };
    }
}
