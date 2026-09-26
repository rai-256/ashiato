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
 *   同じイベントが畳まれずに行を増やす（出来事の時刻は凍結されていて直せない）。
 *   ただし停止が10日を超えたら保持の下限から再開する（本人回答 Q9=c）
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
     * 経過が**数えなかった前進**の累計（[AgeClock.discardedMs]）。
     * **[age] と同じ時計の 2 つ目の口**で、[age] を呼んだ後に読む（落ちた分はそこで数えられる）。
     */
    private val discardedMs: () -> Long,
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
        // **経過を進めてから読む**（数え落とした分は `age()` の中で数えられる）
        val discardedNow = discardedMs()
        val mark = windowStore.load()
        var resumeFloor: Instant? = null
        if (mark != null) {
            // **2 つの時計の進み方を比べる。** 端末の時計が飛んだことは、片方だけでは分からない。
            // **経過が数えなかった前進は差し引く**（本人の決定 C6 / 独立レビュー Important 1）——
            // 起動をまたぐ前進は 30 日で頭打ちに数えられるので、60 日の電源断は
            // 時計が 1 秒も飛んでいなくても 30 日の食い違いを見せる。落ちた分だけを引けば、
            // **その見かけはちょうど 0 になり、本物の飛び（跨ぎの前後で時刻が動いた分）は残る**
            val elapsedMs = (ageNow - mark.ageMs) + (discardedNow - mark.discardedMs)
            val skewMs = Duration.between(mark.end, at).toMillis() - elapsedMs
            if (abs(skewMs) > USAGE_CLOCK_SKEW_TOLERANCE_MS) {
                log(Telemetry.line("usage_clock_skew", source = logicalSource, elapsedMs = skewMs))
                if (!UsageRetention.eventsRetentionExceeded(elapsedMs)) {
                    return CollectionResult.Unavailable(REASON_CLOCK_SKEW)
                }
                // Q9=c: 単調な経過で10日を超えた停止だけ、保持の下限から再開する。
                // mark は先に進めない。諦めた期間の gap も保存できた後で進める。
                resumeFloor = source.retentionFloor(at).clockSkewResumeBegin()
            }
        }
        // **保存した終わりの手前から**（C4）。1 度も取れていなければ、見込みの保持より手前から（C3）
        val begin = resumeFloor ?: mark?.end?.minusMillis(USAGE_WINDOW_OVERLAP_MS)
            ?: at.minusMillis(USAGE_FIRST_WINDOW_MS)
        if (!begin.isBefore(at)) {
            log(Telemetry.line("usage_window_ahead", source = logicalSource))
            return CollectionResult.Unavailable(REASON_WINDOW_AHEAD)
        }
        // **通常の窓を切り詰めない**（spec レビュー R3。Q9=c の自動再開だけが例外）。
        // 見込みの保持（10 日）は API から読めないので、
        // 切ると取得元にまだ残っているイベントを飛ばす。取りこぼした期間の記録は tasks 4.1 の担当
        return when (val read = source.events(CollectionWindow(begin, at))) {
            is EventsResult.Unreadable -> {
                // **窓を進めない。** 0 件と取り違えると、この期間は取り直されないまま消える
                log(Telemetry.line("usage_unreadable", source = logicalSource, error = read.reason))
                CollectionResult.Unavailable(read.reason)
            }
            is EventsResult.Events -> store(read.events, mark, at, ageNow, discardedNow)
        }
    }

    /**
     * 取れた分を記録にして積み、窓の終わりを進める。**0 件でも進める**（本人の決定 C7）。
     *
     * ただし**置き場に 1 件でも書けなかった契機では進めない**（再レビュー round 4）——
     * 書けなかったものは `Outbox` がメモリに抱えるが、端末の空きが尽きたまま続けば手放される。
     * 進めてしまうと、その期間は取り直されないまま取得元の保持を過ぎて消える。
     */
    private fun store(
        events: List<UsageEventSnapshot>,
        mark: UsageWindowMark?,
        at: Instant,
        ageMs: Long,
        discarded: Long,
    ): CollectionResult {
        // 地域も利用者も**1 契機につき 1 度だけ**読む（同じ契機の記録で食い違わせない）
        val collectedIn = zone()
        val user = userId()
        val gap = gapOf(events, mark, at, collectedIn, user)
        val records = gap + events.map {
            it.toIngestRequest(newId(), user, deviceId, collectedIn, labels.label(it.packageName))
        }
        var persisted = 0
        // **その契機で積んだ全件が置き場に書けたか。** 0 件なら真のまま（書くものが無いだけ）
        var allStored = true
        for (record in records) if (outbox.add(record)) persisted++ else allStored = false
        // **書けたときだけ窓を進める**（再レビュー round 4。`HeartbeatEmitter.takeAfter` と同じ形 ——
        // あちらは ST04 の review/code.md R24 で同じ型を 1 度踏んで直してある）。
        //
        // `Outbox.add` が偽で返したものはメモリに載るが、端末の空きが尽きた状態が続けば
        // `MAX_UNWRITTEN` を超えて `lost` として手放される。それでも窓を進めると、
        // **その期間のイベントは二度と取りに行かず**、取得元の保持（見込み 10 日）で消える。
        // gap の記録はもっと悪く、**積み直す経路が無い** —— 窓が進むと次の契機の
        // `floor.excludes(coveredThrough)` が偽になり、そもそも候補にならない。
        //
        // 進めないあいだは同じ期間を 30 分ごとに取り直すが、**それが安全側**（サーバは
        // 内容の鍵で畳む）。やがて窓の始まりが見込みの下限より古くなれば gap が積まれるが、
        // **それは事実として正しい**（そのときには取得元からも消えている）。
        //
        // **0 件だった契機は進める**（本人の決定 C7「0 件でも成功」）—— ここを
        // `persisted == records.size` ではなく `persisted > 0` で判定すると、
        // 携帯を使っていなかっただけの区間で窓が永久に止まる。
        if (allStored) {
            windowStore.save(UsageWindowMark(at, ageMs, discarded))
        } else {
            log(Telemetry.line("usage_window_held", source = logicalSource, count = records.size - persisted))
        }
        // 出すのは件数だけ（製造準備 A-2）。表示名も原文もログに出さない
        log(Telemetry.line("usage", source = logicalSource, count = records.size))
        // **「取れた件数」と「置き場に残せた件数」は別**（位置と同じ規律）。
        // 残せなかった分は `Outbox` がメモリに持って書き直し、持ちきれなければ破棄として報告する
        if (persisted != records.size) {
            log(Telemetry.line("usage_not_persisted", source = logicalSource, count = records.size - persisted))
        }
        return CollectionResult.Collected(records)
    }

    /**
     * 取りに行ったが取得元に無かった期間を 1 件にする（tasks 4.1 / design D4 / 本人の決定 Q4）。
     *
     * 期間は `[既に取れているところ, min(見込みの下限, 返った最古のイベントの時刻))`。
     * 長さが 0 以下なら積まない。
     *
     * **始まりは「保存された窓の終わり」で、問い合わせた窓の始まりではない** ——
     * 窓は重ね幅（[USAGE_WINDOW_OVERLAP_MS]）ぶん手前から問い合わせているが、
     * その 60 秒は**前回の契機で既に取れている**。そこを始まりにすると、
     * 取得元が見込みより長く持っていて最古が保存された終わりちょうどだった場面で
     * 「60 秒だけ取れなかった」という嘘の 1 件が毎回積まれる。
     *
     * **1 度も取れていないときは積まない。** 初回の窓は見込みの 3 倍（[USAGE_FIRST_WINDOW_MS]）
     * まで手を伸ばす探りで、返らなかった分は「収集が動いていたのに取れなかった」ではなく
     * **まだ収集していなかった**期間。積むと出来事の時刻（＝見込みの下限＝導入の 10 日前）が
     * 収集開始日を**そこまで遡らせ**（サーバは記録の最古で `collection_started_on` を下げる）、
     * 導入前の 10 日が「動いていたのに記録が無い」として稼働状況に出る。
     * design D4 が「期間の終わりは必ず収集が動いていた窓の中に入る」と言えるのは、
     * 収集が既に動いていた場合だけ。
     */
    private fun gapOf(
        events: List<UsageEventSnapshot>,
        mark: UsageWindowMark?,
        at: Instant,
        zone: ZoneId,
        user: String,
    ): List<IngestRequest> {
        val coveredThrough = mark?.end ?: return emptyList()
        val floor = source.retentionFloor(at)
        // **窓の始まりが見込みの下限より前か**（design D4）。値そのものはここへ出てこない
        if (!floor.excludes(coveredThrough)) return emptyList()
        // 返っていれば最古で閉じる（取得元が見込みより長く持っていた分は普通に記録になっている）
        val oldest = events.minByOrNull { it.at }?.at
        val end = if (oldest == null) floor.gapEndWhenNothingReturned() else floor.gapEnd(oldest)
        if (!end.isAfter(coveredThrough)) return emptyList()
        // **出すのは件数だけ**（製造準備 A-2 / 独立レビュー R5）。
        // 期間の長さを `elapsed_ms` に載せない —— あの欄は**所要時間**で、
        // 3 日の gap で `elapsed_ms=259200000` を出すと、ログから所要時間を集計したときに壊れる。
        // 期間そのものは記録の `begin` / `end` に入っていて、受け手はそちらを読む
        log(Telemetry.line("usage_gap", source = logicalSource, count = 1))
        return listOf(usageGapRequest(newId(), user, deviceId, zone, coveredThrough, end))
    }

    companion object {
        /** 端末の時計が単調な経過と食い違っている（取得元の統計がずれている疑い） */
        const val REASON_CLOCK_SKEW: String = "clock_skew"

        /** 保存した窓の終わりが今より先にある（問い合わせられる窓にならない） */
        const val REASON_WINDOW_AHEAD: String = "window_ahead"
    }
}
