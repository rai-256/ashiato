// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.content.Context
import java.io.File
import java.io.IOException
import java.time.Instant
import java.time.ZoneId

/**
 * 初回に取り込む粒度と、その順（spec「年・月・週・日」/ 本人の決定 Q5）。
 *
 * **粗いほうから取る。** 年ごとの箱は 2 年ぶん残っていて、取らなければ日が経つごとに消える。
 * 日ごとの箱は 10 日で消えるが、そちらは以後の契機でも取り直される（[ROLLUP_INTERVAL_MS] ごと）ので、
 * 取り込みが途中で終わったときに失われる量が大きいのは粗いほう。
 */
val ROLLUP_IMPORT_ORDER: List<UsageGranularity> = listOf(
    UsageGranularity.YEARLY,
    UsageGranularity.MONTHLY,
    UsageGranularity.WEEKLY,
    UsageGranularity.DAILY,
)

/**
 * 初回の取り込みが終わった後も取り続ける粒度（spec「日ごとの粒度について 6 時間間隔の取得契機ごとに続ける」）。
 *
 * 日ごとの箱だけが 10 日で消える —— 週・月・年は 4 週 / 6 か月 / 2 年ぶん残っているので、
 * 初回に取った後は日ごとの箱が育って自然に繰り上がる。
 */
val ROLLUP_ONGOING_GRANULARITY: UsageGranularity = UsageGranularity.DAILY

/**
 * 集計の問い合わせの始まり。**見込みの保持の下限で切らない**（本人の決定 C3 / spec レビュー R3）。
 *
 * 粒度ごとの見込み（年 2 年 / 月 6 か月 / 週 4 週 / 日 10 日）は AOSP の実装から読んだ値で、
 * **API から読めない**。それで窓を切ると、取得元がそれより長く持っている端末で
 * **まだ残っている箱を飛ばす**（飛ばした分はもう取り直せない）。
 * 取得元は要求した範囲を箱の境界まで広げて返すので、始まりを 0 に置いても
 * **返るのは取得元が持っている分だけ**。費用は粒度ごとに 1 問い合わせ。
 */
val ROLLUP_QUERY_BEGIN: Instant = Instant.EPOCH

/** 取り込み済みの粒度の置き場の名前。**ソースごとに別ファイル**（窓・数えと同じ規律）。 */
fun usageRollupProgressFile(dir: File, logicalSource: String): File =
    File(dir, "usage-rollup-progress-$logicalSource.txt")

/**
 * どの粒度まで取り込んだかを端末の保存領域に置く（spec「取り込みが途中で終わったら
 * 次の契機で取り込んでいない粒度から再開する」）。
 *
 * **インスタンスの中だけに持たない** —— `START_STICKY` の立て直しで消えると、
 * そのたびに 4 粒度を取り直すことになり、取り直した分だけ未送信が膨らむ。
 *
 * 読めなければ**何も取り込んでいないものとして始める**（落とさない）。
 * 失うのは「どこまで取ったか」だけで、取り直した分はサーバ側で畳まれる
 * （同じ集計の原文は毎回同じ文字列になる ＝ tasks 4.4）。
 */
class UsageRollupProgressStore(
    private val file: File,
    private val logicalSource: String,
    private val log: (String) -> Unit,
) {
    fun load(): Set<UsageGranularity> = try {
        if (!file.exists()) {
            emptySet()
        } else {
            // **知らない名前は黙って捨てる** —— 版が上がって粒度が増減しても、
            // 読めた分だけは「取り込み済み」として残る（全部取り直すより軽い）
            file.readLines()
                .mapNotNull { line -> ROLLUP_IMPORT_ORDER.firstOrNull { it.key == line.trim() } }
                .toSet()
        }
    } catch (e: RuntimeException) {
        log(Telemetry.line("usage_rollup_progress_unreadable", source = logicalSource, error = e.javaClass.simpleName))
        emptySet()
    } catch (e: IOException) {
        log(Telemetry.line("usage_rollup_progress_unreadable", source = logicalSource, error = e.javaClass.simpleName))
        emptySet()
    }

    /** 書く。**書いてから差し替える**（[UsageWindowStore] と同じ）。 */
    fun save(done: Set<UsageGranularity>) {
        try {
            file.parentFile?.mkdirs()
            val tmp = File(file.parentFile, "${file.name}.tmp")
            tmp.writeText(ROLLUP_IMPORT_ORDER.filter { it in done }.joinToString("\n") { it.key })
            if (!tmp.renameTo(file)) throw IOException("rename")
        } catch (e: IOException) {
            log(Telemetry.line("usage_rollup_progress_save_failed", source = logicalSource, error = e.javaClass.simpleName))
        }
    }
}

/**
 * アプリ利用の**集計**を [CollectionSource] の口に載せる（tasks 4.3 / design D3 / 本人の決定 Q5）。
 *
 * 1 契機で: まだ取り込んでいない粒度を**年 → 月 → 週 → 日**の順に取り、
 * 返った集計を 1 件 1 記録にして位置と同じ未送信の置き場（C11）へ積み、
 * **取れた粒度だけ**を「取り込み済み」に書く。4 つ揃った後は**日ごとだけ**を取り続ける。
 *
 * **イベント（[AppUsageSourceAdapter]）と別のソースにしてある**のは、粒度の違うものを
 * 1 本に混ぜると内容の鍵での重複の判定と稼働状況の数えが壊れるため（design D3）。
 *
 * **窓を持たない。** イベントと違って集計は「どこまで取ったか」を時刻で数えない ——
 * 取得元は同じ箱を何度でも同じ内容で返すので、毎回まるごと問い合わせて
 * サーバの冪等に畳ませる（design D3。日ごとの箱は 6 時間ごとに 4 回読み直される）。
 * だから**時計が飛んでも取り直せばよく**、イベント側の食い違いの判定は要らない。
 *
 * **取得契機は 6 時間**（[ROLLUP_INTERVAL_MS]）＝生存信号の区間と同じ。24 時間にすると
 * 6 時間の区間に契機が 1 回も入らず、試行 0 / 成功 1 の信号を契約が恒久的に断る（spec レビュー R4）。
 */
class AppUsageRollupSourceAdapter(
    private val source: UsageSource,
    /** 記録の未送信。**位置とイベントと同じもの**（C11） */
    private val outbox: Outbox<IngestRequest>,
    private val progressStore: UsageRollupProgressStore,
    private val labels: AppLabels,
    private val userId: () -> String,
    private val deviceId: String,
    /** 取得時点の端末の地域（C12 / design D7（仮））。**契機ごとに読む** */
    private val zone: () -> ZoneId,
    private val newId: () -> String,
    /** いま取得できる状態か。**本物の判定は tasks 5.2** —— ここは口を埋めるだけ */
    private val capabilityOf: (Context) -> Capability,
    private val log: (String) -> Unit = {},
) : CollectionSource {
    override val logicalSource: String = SourceCadence.APP_USAGE_ROLLUP.logicalSource

    override val intervalMs: Long = SourceCadence.APP_USAGE_ROLLUP.intervalMs

    override fun capability(context: Context): Capability = capabilityOf(context)

    /**
     * その契機ぶんを取り込む。
     *
     * **1 本ずつに絞る**（`@Synchronized`）—— 取り込み済みの印は「読んで、取って、書く」なので、
     * 立て直しの契機と刻みの契機が重なると同じ粒度を 2 回取り、片方の保存がもう片方を追い越す。
     */
    @Synchronized
    override fun collect(window: CollectionWindow): CollectionResult {
        val at = window.end
        val done = progressStore.load()
        val pending = ROLLUP_IMPORT_ORDER.filter { it !in done }
        // 初回が済んでいれば日ごとだけ（spec）。済んでいなければ**残りを続きから**
        val targets = pending.ifEmpty { listOf(ROLLUP_ONGOING_GRANULARITY) }
        // 地域も利用者も**1 契機につき 1 度だけ**読む（同じ契機の記録で食い違わせない）
        val collectedIn = zone()
        val user = userId()

        val enqueued = mutableListOf<IngestRequest>()
        val imported = done.toMutableSet()
        var unreadable: String? = null
        var persisted = 0
        for (granularity in targets) {
            when (val read = source.rollups(granularity, CollectionWindow(ROLLUP_QUERY_BEGIN, at))) {
                is RollupsResult.Unreadable -> {
                    // **取り込み済みにしない。** 0 件と取り違えると、その粒度は二度と取られない
                    log(Telemetry.line("usage_rollup_unreadable", source = logicalSource, error = read.reason))
                    unreadable = read.reason
                }
                is RollupsResult.Rollups -> {
                    for (rollup in read.rollups) {
                        val record = rollup.toIngestRequest(
                            newId(), user, deviceId, collectedIn, granularity, labels.label(rollup.packageName),
                        )
                        enqueued += record
                        if (outbox.add(record)) persisted++
                    }
                    // **0 件でも取り込み済み**（本人の決定 C7）—— 端末に入れたばかりで過去が無いだけ
                    imported += granularity
                }
            }
            // **読めなかったらそこで止める**（次の粒度へ進まない）—— 取得元まるごとが
            // 読めない状態（解錠されていない端末）なので、続けても同じ理由で落ちる
            if (unreadable != null) break
        }
        // **取れた分の印は、途中で止まっても残す**（spec「次の契機で取り込んでいない粒度から再開する」）
        if (imported != done) progressStore.save(imported)
        // 出すのは件数だけ（製造準備 A-2）。表示名も原文も出さない
        log(Telemetry.line("usage_rollup", source = logicalSource, count = enqueued.size))
        if (persisted != enqueued.size) {
            log(
                Telemetry.line(
                    "usage_rollup_not_persisted",
                    source = logicalSource,
                    count = enqueued.size - persisted,
                ),
            )
        }
        // **読めなかった契機は成功に数えない**（イベント側と同じ規律。積めた分は置き場に残る）
        return unreadable?.let { CollectionResult.Unavailable(it) } ?: CollectionResult.Collected(enqueued)
    }
}
