// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.content.Context
import java.time.Instant

/** アプリ利用の論理ソース。登録簿に同じ名前が要る（FR-61 / FR-2）。 */
const val APP_USAGE_LOGICAL_SOURCE: String = "c01-app-usage"

/**
 * アプリ利用の集計の論理ソース（design D3）。
 *
 * **イベントと同じソースに混ぜない** —— 粒度の違うものを 1 本にすると、
 * 内容の鍵での重複の判定と稼働状況の数えが壊れる。
 */
const val APP_USAGE_ROLLUP_LOGICAL_SOURCE: String = "c01-app-usage-rollup"

/** アプリ利用の取得間隔（design D5 / 30 分）。 */
const val USAGE_INTERVAL_MS: Long = 30 * 60 * 1000L

/**
 * 集計の取得間隔（design D3 / 6 時間）。**生存信号の区間と同じ**。
 *
 * 24 時間にすると 6 時間の区間に契機が 1 回も入らず、試行 0 / 成功 1 の生存信号ができて
 * 契約が `invalid_counts` で**恒久的に**断る —— 断られた信号は理由を問わず未送信に残るので、
 * 6 時間ごとに 1 件ずつ端末に積み上がる（spec レビュー R4）。
 */
const val ROLLUP_INTERVAL_MS: Long = 6 * 60 * 60 * 1000L

/**
 * 収集しているソースの刻み（design D5 / specs「ソースごとに独立して収集する」）。
 *
 * **満点の刻みはそのソースの取得間隔**（本人の決定 D5）。位置の 60 秒に固定したままだと、
 * 30 分間隔のアプリ利用の生存信号が 6 時間ごとに「試行 360 / 成功 12」を送り、
 * **取得率 3 % ＝ ずっと眠っていた**と読まれる（独立レビュー R10）。
 *
 * **不変条件: 生存信号の区間 ≥ 取得契機の間隔。** ここで `require` にしてあるので、
 * 破る値のソースは**そもそも組み立てられない**（型の読み込みで落ちる）。
 * 破ると試行 0 / 成功 1 の信号ができ、契約が `invalid_counts` で恒久的に断る。
 *
 * 生存信号の区間は**登録簿の想定間隔のまま**（本人の決定 C8）—— ずらすと
 * 受け手が「想定間隔を超えて何も来ない」と判定する窓とずれ、正常な運用が⑥「途絶」に見える。
 */
enum class SourceCadence(
    val logicalSource: String,
    val intervalMs: Long,
    val heartbeatIntervalMs: Long = HEARTBEAT_INTERVAL_MS,
) {
    LOCATION(LOGICAL_SOURCE, FIX_INTERVAL_MS),
    APP_USAGE(APP_USAGE_LOGICAL_SOURCE, USAGE_INTERVAL_MS),
    APP_USAGE_ROLLUP(APP_USAGE_ROLLUP_LOGICAL_SOURCE, ROLLUP_INTERVAL_MS),
    ;

    init {
        require(intervalMs > 0) { "$logicalSource: 取得契機の間隔が 0 以下" }
        require(heartbeatIntervalMs >= intervalMs) {
            "$logicalSource: 生存信号の区間 $heartbeatIntervalMs ms に取得契機（$intervalMs ms）が 1 回も入らない"
        }
    }
}

/**
 * 1 回の取得が見る範囲 `[begin, end)`。
 *
 * **終わりが始まりより前の窓は組み立てられない** —— 取得元（`UsageStatsManager`）は
 * そういう範囲に `null` を返すので、渡した側は「読めなかった」と「0 件だった」を取り違える。
 */
data class CollectionWindow(val begin: Instant, val end: Instant) {
    init {
        require(!end.isBefore(begin)) { "窓の終わりが始まりより前: $begin .. $end" }
    }
}

/** 1 回の取得の結果。 */
sealed interface CollectionResult {
    /** 取れた。**0 件でも成功**（本人の決定 C7）—— 携帯を使っていなかっただけの区間を失敗にしない。 */
    data class Collected(val records: List<IngestRequest>) : CollectionResult

    /**
     * 取得元が読めなかった。**成功に数えない。**
     * 0 件と取り違えると、その期間は取り直されないまま取得元の保持を過ぎて消える。
     */
    data class Unavailable(val reason: String) : CollectionResult

    /**
     * 取得元が契機を**自分で配る**（位置の Play Services）。記録はその callback の側で積まれ、
     * 数えもそちらが進めるので、親はこの窓について何も積まない。
     */
    data object Streaming : CollectionResult
}

/**
 * 収集の 1 本のソース（design D5）。
 *
 * **`LocationService` は「収集の親」に徹する。** 親は起動を取得条件に依らず行い、
 * 取得条件が欠けたソースは `collect()` を呼ばずに生存信号だけを出す ——
 * **欠けたソースだけを止め、他は取り続ける**（本人の決定 Q7）。
 */
interface CollectionSource {
    /** 登録簿の名前。ログも生存信号もこの名前を名乗る。 */
    val logicalSource: String

    /** 取得契機の間隔。**生存信号の満点の刻みでもある**（`SourceCadence`）。 */
    val intervalMs: Long

    /** いま取得できる状態か。**読むだけで、直そうとしない**（権限の要求は入口の仕事）。 */
    fun capability(context: Context): Capability

    /** その窓のぶんを取る。権限が無いときに投げる例外は**握りつぶさず**親へ抜く。 */
    fun collect(window: CollectionWindow): CollectionResult

    /** 取得を止める。**契機を自分で配るソースだけ**が実際に何かをする。 */
    fun stop() {}
}

/**
 * 位置を [CollectionSource] の口に載せる（tasks 1.1）。**振る舞いは 1 つも変えない** ——
 * 間隔は FR-1 の 60 秒のまま、記録を積むのは今までどおり [FixCollector]。
 *
 * 位置の取得元（Play Services）は**契機を自分で配る**ので、`collect()` がするのは
 * 取得元への登録だけで、結果は [CollectionResult.Streaming]。
 *
 * **呼ばれるたびに登録し直す。** ST06 より前の `onStartCommand` が毎回
 * `fixSource.start(FIX_INTERVAL_MS, callback)` を呼んでいたのと同じ ——
 * アプリを開き直すたびに要求を張り直すのが**復旧の経路**で、
 * 「1 度だけ」に絞ると取得が止まった端末がそこから戻れなくなる。
 * 重ねて呼んでも要求は増えない（`requestLocationUpdates` は**同じ
 * [com.google.android.gms.location.LocationCallback] インスタンス**への要求を置き換える。
 * 親は 1 つの [FixCollector] を作って渡し続ける）。
 */
class LocationSourceAdapter(
    private val fixSource: FixSource,
    private val callback: FixCollector,
) : CollectionSource {
    override val logicalSource: String = SourceCadence.LOCATION.logicalSource
    override val intervalMs: Long = SourceCadence.LOCATION.intervalMs

    override fun capability(context: Context): Capability = androidCapability(context)

    override fun collect(window: CollectionWindow): CollectionResult {
        // **権限が無ければ `SecurityException`。** ここで握りつぶさない ——
        // 親が `no_permission` を残す唯一の経路
        fixSource.start(intervalMs, callback)
        return CollectionResult.Streaming
    }

    override fun stop() {
        fixSource.stop(callback)
    }
}
