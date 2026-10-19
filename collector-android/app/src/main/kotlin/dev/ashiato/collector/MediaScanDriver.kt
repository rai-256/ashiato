// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.util.concurrent.ScheduledExecutorService
import java.util.concurrent.TimeUnit

/** 遅れて 1 回走らせる口。取り消しは戻り値。試験は仮想の時間を渡す。 */
interface MediaScanTimer {
    fun after(delayMs: Long, task: () -> Unit): () -> Unit
}

/** 1 本のスレッドで走る実物。**位置の収集とは別のスレッド**（位置の 60 秒の取得を止めない。D1）。 */
class ExecutorMediaScanTimer(private val executor: ScheduledExecutorService) : MediaScanTimer {
    override fun after(delayMs: Long, task: () -> Unit): () -> Unit {
        val future = executor.schedule({ task() }, delayMs, TimeUnit.MILLISECONDS)
        return { future.cancel(false) }
    }
}

/**
 * 読み手の契機（design D1）: 保存の知らせ（10 秒まとめる）と 6 時間の見直し。
 * 全部の突き合わせの契機（24 時間など）は [MediaScanner.scan] が自分で見る。
 */
class MediaScanDriver(
    private val scanner: MediaScanner,
    private val timer: MediaScanTimer,
    private val debounceMs: Long = OBSERVER_DEBOUNCE_MS,
    private val reviewEveryMs: Long = REVIEW_EVERY_MS,
    /** 1 回の契機ごとの結果の宛先（生存信号の数えと取得可否。D22） */
    private val onOutcome: (MediaScanOutcome) -> Unit = {},
) {
    private var pendingChange: (() -> Unit)? = null
    private var cancelReview: (() -> Unit)? = null
    private var running = false

    /** 始める: 1 度見直し、以降 6 時間ごとに見直す。最初の見直しも契機の口に渡す（呼び手のスレッドを塞がない。D1）。 */
    @Synchronized
    fun start() {
        running = true
        cancelReview = timer.after(0) { review() }
    }

    @Synchronized
    fun stop() {
        running = false
        pendingChange?.invoke()
        pendingChange = null
        cancelReview?.invoke()
        cancelReview = null
    }

    /** 保存の知らせ。続けて来た知らせは 1 回にまとめる。 */
    @Synchronized
    fun onMediaChanged() {
        if (!running) return
        pendingChange?.invoke()
        pendingChange = timer.after(debounceMs) {
            synchronized(this) { pendingChange = null }
            scanner.scan(onOutcome)
        }
    }

    private fun review() {
        try {
            scanner.scan(onOutcome)
        } finally {
            synchronized(this) {
                if (running) cancelReview = timer.after(reviewEveryMs) { review() }
            }
        }
    }
}
