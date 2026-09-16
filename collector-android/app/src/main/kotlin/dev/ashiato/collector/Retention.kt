// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.io.File
import java.io.IOException
import java.time.Instant
import java.time.format.DateTimeParseException

/**
 * 端末の保持の上限（ST04 / FR-8 / FR-9 / NFR-7 / design D15）。**値はここにだけ置く。**
 *
 * 試験だけが差し替える（`Config.overrideForTest` と同じ形）。**本番では常に 90 日 / 2 GB / 83 日。**
 */
data class RetentionPolicy(
    /** 積んでから、この経過を**超えた**記録を捨てる（深掘り Q4） */
    val maxAgeMs: Long = 90 * AgeClock.DAY_MS,
    /** 記録の未送信のバイトがこれを超えたら、積んだ順の古いものから捨てる（C4） */
    val maxBytes: Long = 2L * 1024 * 1024 * 1024,
    /** 最も古い未送信がこの経過に達したら 1 回鳴らす（上限の 7 日前。深掘り Q5） */
    val alertAgeMs: Long = 83 * AgeClock.DAY_MS,
) {
    companion object {
        val DEFAULT = RetentionPolicy()

        @Volatile
        private var override: RetentionPolicy? = null

        val current: RetentionPolicy get() = override ?: DEFAULT

        internal fun overrideForTest(policy: RetentionPolicy) {
            override = policy
        }

        internal fun clearOverrideForTest() {
            override = null
        }
    }
}

/**
 * 保持の上限をかける（ST04 / 深掘り Q1 / Q4 / C1 / C4 / C10 / design D2 / D3 / D13）。
 *
 * - **送れない理由を見ない**（本人の決定 Q1）—— 到達できない・断られ続ける・設定が揃っていない、のどれでも同じ
 * - 90 日は**積んでから**、時計の飛びに影響されない経過で数える（`AgeClock`）
 * - 2 GB は記録の置き場のバイトで数え、積んだ順の古いものから行単位で捨てる
 * - **記録の置き場だけにかける** —— 生存信号と破棄の報告は別の置き場で、ここは触らない（C1 / C2）
 * - 捨てた記録はその場で破棄の報告に数える（`DropLedger`）
 */
class Retention<T : Retainable>(
    private val records: Outbox<T>,
    private val ledger: DropLedger,
    private val age: () -> Long,
    private val policy: () -> RetentionPolicy = { RetentionPolicy.current },
    private val log: (String) -> Unit = {},
) {
    /** 上限をかける。捨てた件数を返す。 */
    @Synchronized
    fun enforce(): Int {
        val p = policy()
        val now = age()
        // 90 日を先に見る（理由が違う破棄は別の報告になる。spec）
        val byAge = dropBy(DropReason.AGE) { commit, rollback ->
            records.dropHead({ now - it.enqAgeMs > p.maxAgeMs }, commit, rollback)
        }
        val byBytes = dropBy(DropReason.BYTES) { commit, rollback ->
            records.dropUntilBytes(p.maxBytes, commit, rollback)
        }
        return byAge + byBytes
    }

    private fun dropBy(
        reason: DropReason,
        run: (commit: (List<Stored<T>>) -> Boolean, rollback: (List<Stored<T>>) -> Unit) -> Int,
    ): Int {
        val touched = LinkedHashSet<String>()
        var before: List<DropDraft>? = null
        val n = run(
            { gone ->
                // **証拠を先に書く**（review R26）。下書きに足して保存できたときだけ、置き場から消させる
                before = ledger.record(reason, gone.map { it.item.logicalSource to instantOf(it.item.eventTime) })
                if (before != null) gone.forEach { touched += it.item.logicalSource }
                before != null
            },
            { _ -> before?.let(ledger::restore) },
        )
        if (n > 0) {
            val remaining = records.oldest()?.item?.eventTime?.let(::instantOf)
            for (source in touched) ledger.endBatch(source, reason, remaining)
            // **件数と理由の種別だけ**（製造準備 A-2）。捨てた記録の位置・時刻の値は出さない
            log(Telemetry.line("retention_dropped", count = n, error = reason.wire))
        }
        return n
    }

    private fun instantOf(s: String): Instant? = try {
        Instant.parse(s)
    } catch (e: DateTimeParseException) {
        null
    }
}

/** 端末の通知の出し先。**試験では Robolectric の本物の NotificationManager を使う。** */
interface RetentionAlerts {
    /** 常駐の通知の本文を差し替える */
    fun ongoing(text: String)

    /** 音の鳴る通知を 1 回出す。出せなかったら（権限が無い）false */
    fun alert(days: Int): Boolean
}

/**
 * 上限が近づいたことを端末で知らせる（ST04 / 深掘り Q5 / design D11（仮））。
 *
 * - 数えるのは**保持の上限の対象になる記録だけ**（生存信号と破棄の報告は数えない。spec R9）
 * - 最も古い未送信が積んでから 1 日以上なら、常駐の通知の本文に日数を出す
 * - 83 日に達したら音の鳴る通知を 1 回。**鳴らした印を小さなファイルに書き**、83 日を下回ったら印を消す
 */
class RetentionNotifier(
    private val records: Outbox<*>,
    private val age: () -> Long,
    private val alerts: RetentionAlerts,
    private val mark: File,
    private val policy: () -> RetentionPolicy = { RetentionPolicy.current },
    private val log: (String) -> Unit = {},
) {
    private var lastText: String? = null
    private var blockedLogged = false

    @Synchronized
    fun update() {
        val p = policy()
        val oldest = records.oldest()
        val elapsed = oldest?.let { age() - it.enqAgeMs }
        val days = elapsed?.let { (it / AgeClock.DAY_MS).toInt() } ?: 0
        val text = if (elapsed != null && elapsed >= AgeClock.DAY_MS) "$BASE_TEXT · 未送信 $days 日" else BASE_TEXT
        if (text != lastText) {
            alerts.ongoing(text)
            lastText = text
        }
        if (elapsed != null && elapsed >= p.alertAgeMs) {
            if (!mark.exists()) {
                if (alerts.alert(days)) {
                    try {
                        mark.parentFile?.mkdirs()
                        mark.writeText(days.toString())
                    } catch (e: IOException) {
                        log(Telemetry.line("retention_mark_failed", error = e.javaClass.simpleName))
                    }
                } else if (!blockedLogged) {
                    // **出せなかったら印を付けない**（review R39）。付けると、権限を戻しても 83 日を下回るまで鳴らない。
                    // ログは 1 度だけ（5 分ごとに積み上げない）
                    log(Telemetry.line("retention_alert_blocked", count = days))
                    blockedLogged = true
                }
            }
        } else if (mark.exists()) {
            // 送れて古い分が無くなった。次の長い圏外ではまた鳴る
            if (!mark.delete()) log(Telemetry.line("retention_mark_delete_failed"))
            blockedLogged = false
        }
    }

    companion object {
        const val BASE_TEXT = "位置を記録しています"
    }
}

/**
 * 送信の契機 1 回ぶん（ST04 / 深掘り Q6 / design D12（仮））。
 *
 * **溜まっている間は間隔を待たずに続けて送る。** ST01 の D9（5 分ごと・1 回 200 件）だと 90 日ぶんに約 55 時間かかり、
 * 古い順に並んでいるので**その間に生まれる新しい記録が NFR-1 の 1 時間を超える**。
 *
 * 続けるのは、記録の送信が 200 か 400 で答えられ、1 件以上を取り除けて、まだ 1 回に載る件数より多く残っているときだけ。
 * **1 件も取り除けなかったら止める** —— 断られ続ける記録だけが溜まっていると、90 日まで止まらない（spec R5）。
 * 続けて送る 1 回ごとに、生存信号と破棄の報告も送る（記録の溜まりが待たせない）。
 */
class Drainer(
    private val records: Sender<IngestRequest>,
    private val recordsOutbox: Outbox<IngestRequest>,
    private val beats: Sender<HeartbeatRequest>,
    private val drops: Sender<DropReport>,
    private val ledger: DropLedger,
    /** 上限の見回りと知らせ。**送る前に**呼ぶ（送れない理由を問わず上限をかける） */
    private val maintenance: () -> Unit = {},
    private val log: (String) -> Unit = {},
    /** 続けて送る回数の上限（止まらない不具合から端末を守るだけ。90 日ぶんは 648 回） */
    private val maxRounds: Int = 100_000,
) {
    /** 1 回の契機。記録を送った回数を返す。 */
    fun tick(): Int {
        var rounds = 0
        while (true) {
            runCatching { maintenance() }.onFailure {
                log(Telemetry.line("maintenance_crashed", error = it.javaClass.simpleName))
            }
            // **記録の送信が落ちても生存信号と破棄の報告は送る**（ST02 と同じ規律）
            val flushed = runCatching { records.flush() }.onFailure {
                log(Telemetry.line("flush_crashed", error = it.javaClass.simpleName))
            }.getOrNull()
            runCatching { beats.flush() }.onFailure {
                log(Telemetry.line("heartbeat_flush_crashed", error = it.javaClass.simpleName))
            }
            // **送信に載せる前に凍結する**（spec「送ろうとした報告は書き換えられない」）
            runCatching {
                ledger.freeze()
                drops.flush()
            }.onFailure {
                log(Telemetry.line("drops_flush_crashed", error = it.javaClass.simpleName))
            }
            rounds++
            val keepGoing = flushed != null && flushed.responded && flushed.removed > 0 &&
                rounds < maxRounds && recordsOutbox.hasMoreThan(MAX_BATCH)
            if (!keepGoing) break
        }
        if (rounds > 1) log(Telemetry.line("drained", count = rounds))
        return rounds
    }
}
