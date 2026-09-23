// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.content.Context
import java.time.Duration
import java.time.Instant
import java.time.ZoneId
import kotlin.math.abs

/**
 * アプリ利用を [CollectionSource] の口に載せる（tasks 3.1〜3.3 / design D1 / D2 / D5）。
 *
 * 1 契機で: 保存した終わりの**手前**から今までを取り、返ったイベントを**1 件 1 記録**にして
 * 位置と同じ未送信の置き場へ積み、**取れたときだけ**窓の終わりを保存する。
 *
 * **親が渡す [CollectionWindow] のうち、使うのは終わり（＝いまの時刻）だけ。**
 * 始まりは「どこまで取ったか」から決まり、それは端末に保存された状態（[UsageWindowStore]）
 * であって親の持ち物ではない —— 親が始まりを決めると、契機を 1 回落としただけで
 * その間のイベントが取り直されないまま消える（取得元の保持は見込み 10 日）。
 * 位置（[LocationSourceAdapter]）が契機ごとの窓をそのまま使えるのは、
 * 取得元が「いまの 1 点」しか返さないから。
 *
 * **窓を進めない場合が 3 つある**（本人の決定 C5 / C6）:
 *
 * - 取得元が「読めなかった」を返した —— 0 件と取り違えて進めると、その期間は取り直されない
 * - 端末の時計が単調な経過と食い違う（[USAGE_CLOCK_SKEW_TOLERANCE_MS]）——
 *   取得元は時計の変化で統計をずらすので、ずれた時刻で取り直すと出来事の時刻が変わった
 *   同じイベントが畳まれずに行を増やす（出来事の時刻は凍結されていて直せない）
 * - 保存した終わりが今より先にある —— 問い合わせられる窓にならない
 *
 * **記録は位置と同じ置き場**（本人の決定 C11）。保持の上限は全ソースを通して古い順にかかる。
 */
class AppUsageSourceAdapter(
    private val source: UsageSource,
    /** 記録の未送信。**位置と同じもの**（C11）—— 分けると保持の上限の意味が変わる */
    private val outbox: Outbox<IngestRequest>,
    private val windowStore: UsageWindowStore,
    private val labels: AppLabels,
    private val userId: () -> String,
    private val deviceId: String,
    /** 取得時点の端末の地域（design D7（仮））。**契機ごとに読む** —— 引っ越しても次の記録から追う */
    private val zone: () -> ZoneId,
    /** 単調な経過（[AgeClock.now]。ST04）。**新しい時計を足さない** */
    private val age: () -> Long,
    /**
     * 起動の世代（[AgeClock.bootGeneration]）。**[age] と同じ時計の 2 つ目の口**で、
     * [age] を呼んだ後に読む（起動の跨ぎはそこで数えられる）。
     */
    private val bootGeneration: () -> Int,
    private val newId: () -> String,
    /**
     * いま取得できる状態か。**本物の判定（`PACKAGE_USAGE_STATS` の付与状態）は tasks 5.2** ——
     * ここは口を埋めるだけで、既定は置かない（既定を置くと、名乗り忘れが「取れている」に化ける）。
     */
    private val capabilityOf: (Context) -> Capability,
    private val log: (String) -> Unit = {},
) : CollectionSource {
    override val logicalSource: String = SourceCadence.APP_USAGE.logicalSource

    override val intervalMs: Long = SourceCadence.APP_USAGE.intervalMs

    override fun capability(context: Context): Capability = capabilityOf(context)

    /**
     * その契機ぶんを取る。
     *
     * **1 本ずつに絞る**（`@Synchronized`）—— 窓は「読んで、取って、書く」なので、
     * 立て直しの契機と刻みの契機が重なると同じ期間を 2 回取り、片方の保存がもう片方を追い越す。
     */
    @Synchronized
    override fun collect(window: CollectionWindow): CollectionResult {
        val at = window.end
        val ageNow = age()
        // **経過を進めてから世代を読む**（起動の跨ぎは `age()` の中で数えられる）
        val generation = bootGeneration()
        val mark = windowStore.load()
        if (mark != null) {
            // **2 つの時計の進み方を比べる。** 端末の時計が飛んだことは、片方だけでは分からない
            val skewMs = Duration.between(mark.end, at).toMillis() - (ageNow - mark.ageMs)
            if (isClockJump(skewMs, generation, mark)) {
                log(Telemetry.line("usage_clock_skew", source = logicalSource, elapsedMs = skewMs))
                return CollectionResult.Unavailable(REASON_CLOCK_SKEW)
            }
        }
        // **保存した終わりの手前から**（C4）。1 度も取れていなければ、見込みの保持より手前から（C3）
        val begin = mark?.end?.minusMillis(USAGE_WINDOW_OVERLAP_MS) ?: at.minusMillis(USAGE_FIRST_WINDOW_MS)
        if (!begin.isBefore(at)) {
            log(Telemetry.line("usage_window_ahead", source = logicalSource))
            return CollectionResult.Unavailable(REASON_WINDOW_AHEAD)
        }
        // **窓を切り詰めない**（spec レビュー R3）—— 見込みの保持（10 日）は API から読めないので、
        // 切ると取得元にまだ残っているイベントを飛ばす。取りこぼした期間の記録は tasks 4.1 の担当
        return when (val read = source.events(CollectionWindow(begin, at))) {
            is EventsResult.Unreadable -> {
                // **窓を進めない。** 0 件と取り違えると、この期間は取り直されないまま消える
                log(Telemetry.line("usage_unreadable", source = logicalSource, error = read.reason))
                CollectionResult.Unavailable(read.reason)
            }
            is EventsResult.Events -> store(read.events, at, ageNow, generation)
        }
    }

    /**
     * その食い違いを「端末の時計が飛んだ」と読んでよいか（本人の決定 C6 / controller の裁定 2026-09-23）。
     *
     * **起動をまたいだ区間の「前へのずれ」は証拠にならない** —— [AgeClock] は起動をまたぐ前進を
     * 壁時計の差から数え、[AgeClock.MAX_REBOOT_GAP_MS]（30 日）で頭打ちにする。
     * だから **60 日 電源を切って放置した端末は、時計が 1 秒も飛んでいなくても
     * 30 日の食い違いを見せる**。それを飛びと読むと、窓は二度と進まない（取れなかった
     * イベントは取得元の保持を過ぎて消える）。跨ぎの前進は数えない。
     *
     * **後ろへのずれは跨いでいても証拠になる** —— 跨ぎの前進は負にならないように
     * 0 で丸められているので、負の食い違いは「壁時計が戻った」ことそのもの。
     * ここまで見逃すと、時刻を戻した端末が壁時計の追いつきとともに
     * ずれた統計を取り直し、出来事の時刻が変わった同じイベントが行を増やす。
     */
    private fun isClockJump(skewMs: Long, generation: Int, mark: UsageWindowMark): Boolean {
        if (abs(skewMs) <= USAGE_CLOCK_SKEW_TOLERANCE_MS) return false
        return generation == mark.generation || skewMs < 0
    }

    /** 取れた分を記録にして積み、窓の終わりを進める。**0 件でも進める**（本人の決定 C7）。 */
    private fun store(
        events: List<UsageEventSnapshot>,
        at: Instant,
        ageMs: Long,
        generation: Int,
    ): CollectionResult {
        // 地域も利用者も**1 契機につき 1 度だけ**読む（同じ契機の記録で食い違わせない）
        val collectedIn = zone()
        val user = userId()
        val records = events.map {
            it.toIngestRequest(newId(), user, deviceId, collectedIn, labels.label(it.packageName))
        }
        var persisted = 0
        for (record in records) if (outbox.add(record)) persisted++
        windowStore.save(UsageWindowMark(at, ageMs, generation))
        // 出すのは件数だけ（製造準備 A-2）。表示名も原文もログに出さない
        log(Telemetry.line("usage", source = logicalSource, count = records.size))
        // **「取れた件数」と「置き場に残せた件数」は別**（位置と同じ規律）。
        // 残せなかった分は `Outbox` がメモリに持って書き直し、持ちきれなければ破棄として報告する
        if (persisted != records.size) {
            log(Telemetry.line("usage_not_persisted", source = logicalSource, count = records.size - persisted))
        }
        return CollectionResult.Collected(records)
    }

    companion object {
        /** 端末の時計が単調な経過と食い違っている（取得元の統計がずれている疑い） */
        const val REASON_CLOCK_SKEW: String = "clock_skew"

        /** 保存した窓の終わりが今より先にある（問い合わせられる窓にならない） */
        const val REASON_WINDOW_AHEAD: String = "window_ahead"
    }
}
