// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import kotlinx.serialization.json.boolean
import kotlinx.serialization.json.jsonPrimitive

/** 測る契機の間隔（ST05 / design D3）。**1 時間**。Doze で遅れるのは受け入れ済み（ST01 の R46）。 */
const val CLOCK_SKEW_INTERVAL_MS: Long = 60 * 60 * 1000L

/** 取れなかったあとの測り直しの間隔（ST05 / design D4（仮））。 */
const val CLOCK_SKEW_RETRY_MS: Long = 5 * 60 * 1000L

/** 1 時間の刻みと測り直しの刻み。**測り直しは専用の刻み**（`ExecutorFlushScheduler` を 2 本）。 */
class ClockTicks(val hourly: FlushScheduler, val retry: FlushScheduler)

/**
 * 端末の測る契機と測り直し（ST05 / design D3 / D4）。
 *
 * - 契機は `start`（起動）・`hourly`（1 時間）・`time_set`（時計の変更）・`retry`（測り直し）
 * - **基準が 1 つも取れなかったら 1 件積み**、5 分ごとの測り直しを始める。測り直しで取れなければ**何も積まない**。
 *   取れたら `retry` で 1 件積んで測り直しをやめる。次の 1 時間の契機は改めて測り、取れなければまた 1 件積む
 * - ログは件数・種別・`available` だけ。時刻の値も差も出さない
 * - 測定の 1 回の失敗は `onCrash` へ渡して外へ出さない（design D11）
 *
 * 3 本の糸（1 時間・測り直し・時計の変更の通知）が入るので、状態は 1 つの錠の中でだけ触る。
 */
class ClockSkewScheduler(
    private val measure: (String) -> IngestRequest,
    private val emit: (IngestRequest) -> Unit,
    private val ticks: ClockTicks,
    private val log: (String) -> Unit = {},
    private val onCrash: (Throwable) -> Unit = {},
) {
    private val lock = Any()
    private var retrying = false

    /**
     * 止めた印（review R3）。`shutdownNow()` は錠を待っている糸を止めないので、`stop()` の後に
     * 錠を取った測定がここで引き返す。引き返さないと記録を積み、測り直しの刻みを新しく立ててしまう。
     */
    private var stopped = false

    /** その場で 1 回測り（`start`）、1 時間の刻みを立てる。 */
    fun start() {
        synchronized(lock) { stopped = false }
        run(TRIGGER_START)
        ticks.hourly.every(CLOCK_SKEW_INTERVAL_MS) { run(TRIGGER_HOURLY) }
    }

    /** 端末の時計が変更された。 */
    fun timeChanged() = run(TRIGGER_TIME_SET)

    fun stop() = synchronized(lock) {
        ticks.hourly.cancel()
        ticks.retry.cancel()
        retrying = false
        stopped = true
    }

    private fun run(trigger: String) {
        runCatching { synchronized(lock) { measureLocked(trigger) } }.onFailure(onCrash)
    }

    private fun measureLocked(trigger: String) {
        if (stopped) return
        // 測り直しを畳んだ後に錠を待っていた測り直しは測らない（review R3）。
        // 測ると余分な `retry` の記録が積まれ、D4 の反転条件の比を汚す
        if (trigger == TRIGGER_RETRY && !retrying) return
        val record = measure(trigger)
        val available = record.payload["available"]!!.jsonPrimitive.boolean
        if (!available && trigger == TRIGGER_RETRY) return
        emit(record)
        log("kind=clock_skew_measured source=$CLOCK_LOGICAL_SOURCE count=1 available=$available")
        when {
            available -> stopRetrying()
            !retrying -> {
                retrying = true
                ticks.retry.every(CLOCK_SKEW_RETRY_MS) { run(TRIGGER_RETRY) }
            }
        }
    }

    private fun stopRetrying() {
        if (!retrying) return
        retrying = false
        ticks.retry.cancel()
    }

    companion object {
        const val TRIGGER_START = "start"
        const val TRIGGER_HOURLY = "hourly"
        const val TRIGGER_TIME_SET = "time_set"
        const val TRIGGER_RETRY = "retry"
    }
}
