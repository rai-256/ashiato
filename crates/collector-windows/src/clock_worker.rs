// SPDX-License-Identifier: AGPL-3.0-only
//! 基準の読み取りを見回りの輪の外で行う作業スレッド（design D7 / D11）。
//!
//! `/healthz`（最長で要求の打ち切りまで）と `w32tm`（5 秒で打ち切り。止まっていれば続けて `wevtutil` も 5 秒）は、見回りの輪の中で待つと
//! その間の前景の切り替えを取りこぼす。**1 本の作業スレッドが両方を読み、結果を通り道で返す。**
//! 見回りは毎回 `poll` で覗くだけで、待たない。
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;

use crate::clock::{ReferenceClock, ReferenceReading, Uptime};
use crate::telemetry;
use crate::time_sync::{TimeSyncReading, TimeSyncSource};

/// 読んだ時刻同期の状態と、それを読む直前と直後の起動からの経過時間。
#[derive(Debug)]
pub struct TimedTimeSync {
    pub reading: TimeSyncReading,
    pub uptime_before_ms: u64,
    pub uptime_after_ms: u64,
}

/// 作業スレッドが返す 1 回分の読み取り。取れなかったものは**理由の文字列**で持つ。
#[derive(Debug)]
pub struct ClockReading {
    /// 基準の出どころ（`ReferenceClock::source`。取り込み口の `host:port`）。
    pub source: String,
    /// 取り込み口の応答から読んだ基準。
    pub reference: Result<ReferenceReading, String>,
    /// Windows の時刻同期の状態。
    pub time_sync: Result<TimedTimeSync, String>,
}

impl ClockReading {
    /// 作業スレッドが結果を返さなかった（`worker_failed`）。2 つとも取れなかったとして扱う。
    pub fn worker_failed(source: String) -> Self {
        Self {
            source,
            reference: Err("worker_failed".into()),
            time_sync: Err("worker_failed".into()),
        }
    }
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
    pub fn spawn(
        reference: Arc<dyn ReferenceClock>,
        time_sync: Arc<dyn TimeSyncSource>,
        uptime: Arc<dyn Uptime>,
    ) -> Self {
        let (tx, rx) = mpsc::channel();
        let _ = std::thread::Builder::new()
            .name("clock-skew-reader".into())
            .spawn(move || {
                let source = reference.source();
                let reference = reference
                    .now()
                    .map_err(|e| telemetry::error_kind(&e).to_string());
                let uptime_before_ms = uptime.millis();
                let read = time_sync.read();
                // 刻みで切り捨てた分を直後の側へ足す（`HttpDateClock` と同じ。D9（仮））
                let uptime_after_ms = uptime.millis() + uptime.resolution_ms() - 1;
                let time_sync = read
                    .map(|reading| TimedTimeSync {
                        reading,
                        uptime_before_ms,
                        uptime_after_ms,
                    })
                    .map_err(|e| e.reason());
                let reading = ClockReading {
                    source,
                    reference,
                    time_sync,
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
