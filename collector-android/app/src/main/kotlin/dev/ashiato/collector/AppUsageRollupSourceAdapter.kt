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
        writeAtomically(file, logicalSource, "usage_rollup_progress_save_failed", log) {
            ROLLUP_IMPORT_ORDER.filter { it in done }.joinToString("\n") { it.key }
        }
    }
}

/**
 * 書いてから差し替える（[UsageWindowStore] と同じ）。**差し替えに失敗したら書きかけを片付ける**
 * （独立レビュー R7）—— `rename` が偽を返す経路は自分の `catch` に入って戻るので、
 * `finally` で消さないと `*.tmp` が置き場に残る。
 */
private fun writeAtomically(
    file: File,
    logicalSource: String,
    failureKind: String,
    log: (String) -> Unit,
    text: () -> String,
) {
    val tmp = File(file.parentFile, "${file.name}.tmp")
    try {
        file.parentFile?.mkdirs()
        tmp.writeText(text())
        if (!tmp.renameTo(file)) throw IOException("rename")
    } catch (e: IOException) {
        log(Telemetry.line(failureKind, source = logicalSource, error = e.javaClass.simpleName))
    } finally {
        // 差し替わっていれば既に無い（`delete` が偽を返すだけ）。残っていれば書きかけなので消す
        tmp.delete()
    }
}

/**
 * 既に積んだ集計を覚えておく幅（独立レビュー R3）。
 *
 * 日ごとの箱は取得元に 10 日ぶん残っていて、6 時間ごとの契機が**毎回まるごと**返させる。
 * 覚えていないと、同じ 1 件が寿命のあいだに約 40 回、位置と同じ未送信の置き場（C11）へ積まれる。
 *
 * 幅は見込みの保持（日ごと 10 日）の 3 倍。[USAGE_FIRST_WINDOW_MS] と同じ考えで、
 * **見込みが外れて取得元が長く持っていても覚えていられる**ようにしてある ——
 * ここで足りないと、その分は取りこぼしではなく**重複**になる（失われるものは無い）。
 *
 * **反転条件**: 端末の置き場がこの台帳で膨らむ実測（tasks 7.1）が出たら狭める。
 */
const val ROLLUP_SEEN_WINDOW_MS: Long = 30 * 24 * 60 * 60 * 1000L

/**
 * 覚えておく件数の上限。**無制限に育てない**（独立レビュー R3）。
 *
 * 1 日に利用のあるアプリを 150 本と見ても 30 日で 4,500 件なので、
 * 普通の端末では当たらない。当たったときは**古い箱から忘れる** ——
 * 忘れた箱が返れば積み直すだけで、**失われるものは無い**（サーバは内容の鍵で畳む）。
 */
const val ROLLUP_SEEN_MAX: Int = 10_000

/** 既に積んだ集計の台帳の置き場の名前。**ソースごとに別ファイル**。 */
fun usageRollupSeenFile(dir: File, logicalSource: String): File =
    File(dir, "usage-rollup-seen-$logicalSource.txt")

/**
 * 既に積んだ集計の原文の指紋を覚えておく（独立レビュー R3）。
 *
 * **窓を切り詰める代わりにここで止める** —— 問い合わせの窓を見込みの下限で切るのは
 * C3 と spec レビュー R3 が明示で禁じている（取得元がそれより長く持っている端末で、
 * まだ残っている箱を飛ばす）。切ってよいのは**積む側**で、そこで落としても
 * 取得元のデータは何も失われない。
 *
 * **覚えるのは原文そのものではなく指紋**（SHA-256 の先頭 128 ビット）。
 * 原文を持つと台帳が記録と同じ大きさになり、置き場を二重に食う。
 * 冪等キーは `logical_source` + `event_time` + `raw` から作られ、
 * このソースの中では `event_time` も原文の `end` に入っているので、**原文だけで一意に決まる**。
 *
 * **今日の箱は育つので指紋が変わる** —— そのときは別の 1 件として積む（鍵が変わるので畳まれない）。
 *
 * **インスタンスの中だけに持たない** —— `START_STICKY` の立て直しで消えると、
 * そのたびに 10 日ぶんを積み直す。読めなければ**何も積んでいないものとして始める**（落とさない）。
 */
class UsageRollupSeenStore(
    private val file: File,
    private val logicalSource: String,
    private val log: (String) -> Unit,
) {
    /** 指紋 → その箱の終わり（忘れる順を決めるためだけに持つ）。 */
    fun load(): MutableMap<String, Instant> = try {
        if (!file.exists()) {
            LinkedHashMap()
        } else {
            val out = LinkedHashMap<String, Instant>()
            for (line in file.readLines()) {
                // 壊れた行は黙って捨てる（捨てた分は積み直されるだけで、失われるものは無い）
                val parts = line.trim().split(" ")
                if (parts.size != 2) continue
                val at = parts[1].toLongOrNull() ?: continue
                out[parts[0]] = Instant.ofEpochMilli(at)
            }
            out
        }
    } catch (e: RuntimeException) {
        log(Telemetry.line("usage_rollup_seen_unreadable", source = logicalSource, error = e.javaClass.simpleName))
        LinkedHashMap()
    } catch (e: IOException) {
        log(Telemetry.line("usage_rollup_seen_unreadable", source = logicalSource, error = e.javaClass.simpleName))
        LinkedHashMap()
    }

    fun save(seen: Map<String, Instant>) {
        writeAtomically(file, logicalSource, "usage_rollup_seen_save_failed", log) {
            seen.entries.joinToString("\n") { "${it.key} ${it.value.toEpochMilli()}" }
        }
    }
}

/**
 * 集計を[忘れる][ROLLUP_SEEN_WINDOW_MS]。**新しい箱から残す** ——
 * 件数で溢れたときに古いほうを落とすのは、古い箱ほど取得元から先に消えて二度と返らないから。
 */
internal fun pruneRollupSeen(seen: MutableMap<String, Instant>, now: Instant): MutableMap<String, Instant> {
    val floor = now.minusMillis(ROLLUP_SEEN_WINDOW_MS)
    val kept = seen.entries
        .filter { !it.value.isBefore(floor) }
        .sortedByDescending { it.value }
        .take(ROLLUP_SEEN_MAX)
    if (kept.size == seen.size) return seen
    val out = LinkedHashMap<String, Instant>(kept.size)
    for (entry in kept) out[entry.key] = entry.value
    return out
}

/** 原文の指紋（SHA-256 の先頭 128 ビット）。**原文そのものは台帳に書かない。** */
internal fun rollupFingerprint(raw: String): String =
    java.security.MessageDigest.getInstance("SHA-256")
        .digest(raw.toByteArray(Charsets.UTF_8))
        .take(16)
        .joinToString("") { "%02x".format(it) }

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
    /**
     * 既に積んだ集計の台帳（独立レビュー R3）。**窓を切り詰める代わりにここで止める** ——
     * 日ごとの箱は 10 日ぶんが 6 時間ごとに毎回まるごと返るので、覚えていないと
     * 同じ 1 件が寿命のあいだに約 40 回、位置と同じ置き場へ積まれる。
     */
    private val seenStore: UsageRollupSeenStore,
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

        val seen = seenStore.load()
        val seenBefore = seen.size

        val enqueued = mutableListOf<IngestRequest>()
        val imported = done.toMutableSet()
        var unreadable: String? = null
        var persisted = 0
        var skipped = 0
        var notImported = 0
        for (granularity in targets) {
            when (val read = source.rollups(granularity, CollectionWindow(ROLLUP_QUERY_BEGIN, at))) {
                is RollupsResult.Unreadable -> {
                    // **取り込み済みにしない。** 0 件と取り違えると、その粒度は二度と取られない
                    log(Telemetry.line("usage_rollup_unreadable", source = logicalSource, error = read.reason))
                    unreadable = read.reason
                }
                is RollupsResult.Rollups -> {
                    // **その粒度で積むべき全件が置き場に書けたか**（再レビュー round 3）。
                    // 0 件なら真のまま（書くものが無いだけ）
                    var allStored = true
                    for (rollup in read.rollups) {
                        val record = rollup.toIngestRequest(
                            newId(), user, deviceId, collectedIn, granularity, labels.label(rollup.packageName),
                        )
                        // **既に積んだ原文は積み直さない**（独立レビュー R3）。
                        // 育っている途中の箱は原文が変わるので、そのときは別の 1 件として積む
                        val print = rollupFingerprint(record.raw)
                        if (seen.containsKey(print)) {
                            skipped++
                            continue
                        }
                        enqueued += record
                        // **置き場に書けたときだけ台帳へ入れる**（再レビュー F1）——
                        // `Outbox.add` が偽で返したものはメモリに載るが、端末の空きが尽きた状態が続けば
                        // `MAX_UNWRITTEN` を超えて `lost` として手放される。先に台帳へ入れると、
                        // **確定済みの箱は原文が変わらないので指紋も変わらず、二度と積まれない**。
                        // 積み直しは冪等で安全（サーバが内容の鍵で畳む）だが、積まないのは取りこぼし ——
                        // `unwritten` に同じ原文が重なる副作用は受け入れる。
                        // **黙って永久に消えるほうを選ばない。**
                        if (outbox.add(record)) {
                            seen[print] = rollup.lastAt
                            persisted++
                        } else {
                            allStored = false
                        }
                    }
                    // **積んだ全件が書けたときだけ「取り込み済み」にする**（再レビュー round 3）。
                    //
                    // spec は逐語で「年と月の粒度まで**取り込んだ**ところで収集が止まり」と書いており、
                    // 取得元から**読めた**ことは取り込んだことではない（取り込みは未送信に積むところまで）。
                    // 読めただけで印を付けると、置き場が満杯の端末で**年・月・週は二度と読まれない**
                    // —— 初回の 1 度きりだからで、年は 2 年ぶんが取得元からも日ごとに消えていく
                    // （`loss: uncaptured`）。日ごとは以後の契機で取り直されるので助かるが、粗いほうは助からない。
                    //
                    // 置き場が満杯の端末では年の粒度が毎契機読み直されるが、**それが安全側** ——
                    // 積み直しは冪等でサーバが内容の鍵で畳む。積まないことは取りこぼし。
                    //
                    // **0 件が返った粒度は取り込み済みにする**（本人の決定 C7「0 件でも成功」）。
                    // 書くものが無いだけで失敗ではない —— ここを `persisted == 0` で判定すると、
                    // 過去が無い端末で 4 粒度が永久に読み直される。
                    if (allStored) imported += granularity else notImported++
                }
            }
            // **読めなかったらそこで止める**（次の粒度へ進まない）—— 取得元まるごとが
            // 読めない状態（解錠されていない端末）なので、続けても同じ理由で落ちる
            if (unreadable != null) break
        }
        // **取れた分の印は、途中で止まっても残す**（spec「次の契機で取り込んでいない粒度から再開する」）
        if (imported != done) progressStore.save(imported)
        // **台帳が動いたときだけ書く。** 動く道は「書けた分を足した」と「忘れる幅で落とした」の 2 つだけ
        val pruned = pruneRollupSeen(seen, at)
        if (persisted > 0 || pruned.size != seenBefore) seenStore.save(pruned)
        // 出すのは件数だけ（製造準備 A-2）。表示名も原文も出さない
        log(Telemetry.line("usage_rollup", source = logicalSource, count = enqueued.size))
        if (notImported > 0) {
            // **印を付けずに残した粒度の数**（次の契機で読み直す）。置き場が書けない端末で立つ
            log(Telemetry.line("usage_rollup_not_imported", source = logicalSource, count = notImported))
        }
        if (skipped > 0) {
            // **積み直さずに済んだ件数**（tasks 7.1 の実測が読む）
            log(Telemetry.line("usage_rollup_already_sent", source = logicalSource, count = skipped))
        }
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
