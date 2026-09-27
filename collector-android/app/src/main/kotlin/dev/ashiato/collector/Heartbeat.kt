// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.time.Instant
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive

/**
 * 生存信号の間隔。**登録簿の想定間隔（FR-35 の初期値 = 位置 6 時間）に合わせる**（tasks 7.1）。
 *
 * 別の値にすると、受け手が「想定間隔を超えて何も来ない」と判定する窓とずれ、
 * 正常な運用が⑥「途絶」に見える（FR-80）。
 *
 * **電池は問題にならない。** 位置は FR-1 が 60 秒間隔で取っており、
 * 6 時間に 1 回はその 360 分の 1 未満（深掘り「確かめたが問わなかったこと」）。
 */
const val HEARTBEAT_INTERVAL_MS: Long = 6 * 60 * 60 * 1000L

/**
 * 生存信号 1 件。**Rust 側（`crates/server/src/heartbeat.rs`）と同じ形**でなければならない。
 * 契約の正典は `docs/collector-contract.md`。
 *
 * **冪等キーはここでは作らない。** 鍵の算出はサーバ側だけが行い、
 * `logical_source` + `emitted_at` + `raw` から決まる —— だから
 * **再送しても同じ 1 件になる**（積んだ 1 件をそのまま送り直すため）。
 */
@Serializable
data class HeartbeatRequest(
    override val id: String,
    @SerialName("user_id") val userId: String,
    @SerialName("logical_source") override val logicalSource: String,
    @SerialName("device_id") val deviceId: String? = null,
    /** 収集側が信号を作った時刻。**受信時刻ではない**（第 6 回 Q24 と同じ向き） */
    @SerialName("emitted_at") val emittedAt: String,
    /** そのソースを取得できる状態か（権限・センサ・接続。深掘り Q5） */
    val capturable: Boolean,
    /** 取得できないとき、何が満たされていないか。**空のまま `capturable=false` は断られる** */
    val blockers: List<String> = emptyList(),
    /** 前回の信号からの取得の試行回数（第 5 回 Q17） */
    val attempts: Int,
    /** そのうち成功した回数（同上） */
    val successes: Int,
    /** 原文。**素通しで残る**ので、同じ 1 件は毎回同じ文字列でなければならない */
    val raw: String,
) : Outboxable

/**
 * 取得できる状態かどうかと、その理由。
 *
 * **理由を持たない「取れない」は受け付けられない**（specs）——
 * 「取れない状態だった」とだけ残っても、権限なのかセンサなのか接続なのかが分からなければ、
 * 扉 #14 が求めた区別に届かない。
 */
data class Capability(val capturable: Boolean, val blockers: List<String>) {
    init {
        // **ここで落とす。** 理由の無い「取れない」を組み立てられる口を残すと、
        // サーバに断られるまで誰も気付かない（未送信に居座り続ける）
        require(capturable || blockers.isNotEmpty()) { "取れない理由が無い" }
    }

    companion object {
        /** 満たされていないものの名前。**この 3 つが specs の「権限・センサ・接続」にあたる。** */
        const val PERMISSION = "permission"
        const val SENSOR = "sensor"
        const val NETWORK = "network"

        /**
         * 3 つの状態から組み立てる。**1 つでも欠けていれば「取れない」。**
         *
         * 接続が無いだけで「取れない」にするのは、位置の取得そのものは網に依存しないので
         * 行きすぎに見えるが、**送れない期間の記録は端末内にしか無い**（ST04 の保持上限を
         * 超えれば破棄される）ので、取得の健全性と同じ列に載せる。
         */
        fun of(permission: Boolean, sensor: Boolean, network: Boolean): Capability {
            val blockers = buildList {
                if (!permission) add(PERMISSION)
                if (!sensor) add(SENSOR)
                if (!network) add(NETWORK)
            }
            return Capability(blockers.isEmpty(), blockers)
        }
    }
}

/**
 * 生存信号に載る 1 区間ぶんの数え（tasks 1.3 / spec レビュー R4）。
 *
 * **`successes <= attempts` を型で壊せなくする。** 契約（`docs/collector-contract.md`）は
 * 成功が試行を超える信号を `invalid_counts` で**恒久的に**断り、
 * 断られた信号は理由を問わず未送信に残る —— 作れてしまうと端末に永久に居座る。
 */
data class Counts(val attempts: Int, val successes: Int) {
    init {
        require(attempts >= 0) { "試行が負: $attempts" }
        require(successes >= 0) { "成功が負: $successes" }
        require(successes <= attempts) { "成功 $successes が試行 $attempts を超えている" }
    }
}

/**
 * 前回の生存信号からの取得の試行と成功を数える（第 5 回 Q17。tasks 7.2b）。
 *
 * **これが ST01 の R46（Doze）が渡した宿題の答え。** 生存信号は 6 時間間隔なので
 * 実測の 14 分の空きの中には入らないが、**試行と成功の数なら信号 1 件でその区間の
 * 取得率が残る** —— 信号が来ている＝生きていた / 取得率が低い＝眠っていた /
 * 信号が来ない＝死んでいた、の 3 つが分かれる。
 *
 * **試行は経過時間から引く。** Play Services は「取れなかった」を報告しないので、
 * 数えられるのは成功だけ —— 失敗の契機は起きないのではなく、**呼ばれない**
 * （Doze の maintenance window の外では callback ごと止まる）。
 * 区間の長さを取得間隔（FR-1 の 60 秒）で割った数が「満点」にあたり、
 * 6 時間なら 360 回になる（design D3 が同じ数を挙げている）。
 *
 * **新しい契機を起こさないので電池は増えない** —— 数えるのは成功したときと、
 * 生存信号を組み立てるときの 2 か所だけ。
 */
class AttemptCounters(
    private val now: () -> Instant,
    /**
     * 満点の刻み。**そのソースの取得間隔**（`SourceCadence`。位置 60 秒 / アプリ利用 30 分 /
     * 集計 6 時間）。**既定値を置かない**（tasks 1.3 / 独立レビュー R10）—— 位置の 60 秒に
     * 固定したままだと、30 分間隔のアプリ利用が 6 時間ごとに「試行 360 / 成功 12」を送り、
     * **取得率 3 % ＝ ずっと眠っていた**と読まれる。
     */
    private val intervalMs: Long,
    /**
     * 数えの置き場。**プロセスの立て直しをまたいで残す**（review/code.md の R16 / C-2 / I8）。
     *
     * `Outbox` は「インスタンスの中だけに積むと立て直しで無言で消える」を理由に
     * `FileOutboxStore` に落としたのに、**同じ理由が当てはまる数えは落とされていなかった**。
     * `since` も一緒に新品になるので、**死んでいた区間そのものが観測から落ちる** ——
     * 6 時間のうち 5 時間 50 分死んで 10 分前に立て直されると、
     * 次の信号は `10 / 10` で「取得率 100 %」になり、画面には「健全」と出る。
     * それは第 5 回 Q17（ST01 の R46 の宿題の答え）が見分けようとした当の区間。
     */
    private val store: CounterStore = MemoryCounterStore(),
) {
    private val restored = store.load()
    private var since: Instant = restored?.first ?: now()
    private var successes: Int = restored?.second ?: 0

    /** 取得できた。**契機ごとに 1 回**呼ぶ。 */
    @Synchronized
    fun recordSuccess() {
        successes++
        store.save(since, successes)
    }

    /** いまの数え。**読むだけでは戻さない**（信号を組み立てられなかったときに数えが消える）。 */
    @Synchronized
    fun peek(): Counts = Counts(attemptsNow(), successes)

    /**
     * 数えを取り出して**戻す**（specs「数えは信号を送るたびに戻る」）。
     *
     * 戻さないと区間の取得率ではなく「導入以来の累計」になり、
     * 眠っていた区間が薄まって見えなくなる。
     */
    /**
     * 数えを取り出して**戻す**（specs「数えは信号を送るたびに戻る」）。
     *
     * **`commit` が返るまで戻さない**（review/code.md の R24 / H-7 / F9 / I9）。
     * 積めなかった信号のぶんまで数えが消えると、その区間の取得率が丸ごと失われる ——
     * `peek()` の docstring が書いていた問題そのものが、`emit()` 側で起きていた。
     */
    @Synchronized
    fun <T> takeAfter(commit: (Counts) -> Pair<Boolean, T>): Pair<Boolean, T> {
        val got = Counts(attemptsNow(), successes)
        val (stored, value) = commit(got)
        if (stored) {
            since = now()
            successes = 0
            store.save(since, successes)
        }
        return stored to value
    }

    @Synchronized
    fun take(): Counts {
        val got = Counts(attemptsNow(), successes)
        since = now()
        successes = 0
        store.save(since, successes)
        return got
    }

    /**
     * 区間の満点。**成功を下回らせない** —— 端末時計が戻ると
     * 「成功が試行を超える」信号になり、受け口に断られて未送信に居座る（2 巡目 R7）。
     */
    private fun attemptsNow(): Int {
        val elapsedMs = java.time.Duration.between(since, now()).toMillis()
        val expected = if (elapsedMs <= 0) 0 else (elapsedMs / intervalMs).toInt()
        return maxOf(expected, successes)
    }
}

/**
 * 数えの置き場。**既定はメモリだけ**にせず、本番は必ずファイルを渡す
 * （`Outbox` が `OutboxStore` に既定を持たせなかったのと同じ理由）。
 */
interface CounterStore {
    fun load(): Pair<Instant, Int>?

    fun save(since: Instant, successes: Int)
}

/**
 * 置き場を持たない入れ物。**`object` にしない** —— 単一のインスタンスを既定にすると、
 * 同じプロセスの別の数えが同じ値を共有する（試験どうしが混ざり、本番でも
 * ソースが 2 つになった日に混ざる）。
 */
class MemoryCounterStore : CounterStore {
    private var value: Pair<Instant, Int>? = null

    override fun load(): Pair<Instant, Int>? = value

    override fun save(since: Instant, successes: Int) {
        value = since to successes
    }
}

/**
 * 数えを端末の保存領域に置く。形は 1 行 `<ISO8601> <successes>`。
 *
 * **読めなくても落とさない** —— 数えは証拠ではなく目安で、失っても記録は消えない。
 * ただし**失ったことは残す**（黙って 0 から始めない）。
 */
class FileCounterStore(
    private val file: java.io.File,
    /** この置き場が数えているソース。**ログはその名を名乗る**（tasks 1.2）。 */
    private val logicalSource: String,
    private val log: (String) -> Unit,
) : CounterStore {
    override fun load(): Pair<Instant, Int>? = try {
        if (!file.exists()) {
            null
        } else {
            val parts = file.readText().trim().split(" ")
            Instant.parse(parts[0]) to parts[1].toInt()
        }
    } catch (e: RuntimeException) {
        log(Telemetry.line("counters_unreadable", source = logicalSource, error = e.javaClass.simpleName))
        null
    } catch (e: java.io.IOException) {
        log(Telemetry.line("counters_unreadable", source = logicalSource, error = e.javaClass.simpleName))
        null
    }

    override fun save(since: Instant, successes: Int) {
        try {
            file.writeText("$since $successes")
        } catch (e: java.io.IOException) {
            log(Telemetry.line("counters_save_failed", source = logicalSource, error = e.javaClass.simpleName))
        }
    }
}

/** ST06 より前の、ソースが 1 本だけだった時代の数えの置き場。 */
const val LEGACY_COUNTERS_FILE: String = "heartbeat-counters.txt"

/**
 * 数えの置き場の名前。**ソースごとに別ファイル**（tasks 1.3 / 独立レビュー R10）——
 * 2 本目が同じ名前を開くと、互いの `since` と成功数を潰し合う。
 */
fun counterStoreFile(dir: java.io.File, logicalSource: String): java.io.File =
    java.io.File(dir, "heartbeat-counters-$logicalSource.txt")

/**
 * ST06 より前の 1 本（[LEGACY_COUNTERS_FILE]）を**位置の名前へ移す**。移した先を返す。
 *
 * **位置以外へは引き継がない** —— 位置の数えを別のソースの取得率として読ませない。
 * 移せなくても落とさない（数えは証拠ではなく目安で、失っても記録は消えない）。
 * 読めなければ `FileCounterStore` が新品から始める。
 */
fun migrateLegacyCounters(dir: java.io.File, logicalSource: String): java.io.File {
    val target = counterStoreFile(dir, logicalSource)
    if (logicalSource == LOGICAL_SOURCE && !target.exists()) {
        val legacy = java.io.File(dir, LEGACY_COUNTERS_FILE)
        if (legacy.exists()) runCatching { legacy.renameTo(target) }
    }
    return target
}

/**
 * 生存信号を組み立てて未送信へ積む（specs/device-collection）。
 *
 * **記録の生成に相乗りさせない**（tasks 7.4）—— 記録が 1 件も生成されない期間に
 * 稼働を残すことが FR-78 の目的そのものなので、記録の契機から呼ぶと意味が消える。
 */
class HeartbeatEmitter(
    private val outbox: Outbox<HeartbeatRequest>,
    private val counters: AttemptCounters,
    private val userId: String,
    private val deviceId: String,
    private val logicalSource: String,
    private val capability: () -> Capability,
    private val now: () -> Instant,
    private val newId: () -> String,
    private val log: (String) -> Unit = {},
) {
    /** 1 件を積む。積めたかを返す。 */
    fun emit(): Boolean {
        val cap = capability()
        val at = now()
        // **積めてから数えを戻す**（review/code.md の R24）。先に `take()` していたときは、
        // 置き場へ書けなかった区間の取得率が丸ごと消えていた。
        val (stored, _) = counters.takeAfter { counts ->
            buildAndStore(cap, at, counts.attempts, counts.successes) to Unit
        }
        return stored
    }

    private fun buildAndStore(cap: Capability, at: Instant, attempts: Int, successes: Int): Boolean {
        // **原文は組み立てた値そのもの。** 素通しで残るので、同じ 1 件は毎回同じ文字列になる
        // （冪等キーがこの文字列から作られる）。
        val fields = mapOf(
            "alive" to JsonPrimitive(true),
            "capturable" to JsonPrimitive(cap.capturable),
            "blockers" to JsonPrimitive(cap.blockers.joinToString(",")),
            "attempts" to JsonPrimitive(attempts),
            "successes" to JsonPrimitive(successes),
            "emitted_at" to JsonPrimitive(at.toString()),
            "device_id" to JsonPrimitive(deviceId),
        )
        val request = HeartbeatRequest(
            id = newId(),
            userId = userId,
            logicalSource = logicalSource,
            deviceId = deviceId,
            emittedAt = at.toString(),
            capturable = cap.capturable,
            blockers = cap.blockers,
            attempts = attempts,
            successes = successes,
            raw = ingestJson.encodeToString(JsonObject(fields)),
        )
        val stored = outbox.add(request)
        // 出すのは種別と件数だけ。**取得できない理由は私的データではない**ので出せる
        log(Telemetry.line("heartbeat", source = logicalSource, count = attempts, error = cap.blockers.firstOrNull()))
        // 積めなかったら**数えは戻らない**（`takeAfter`）。次の契機でまとめて載る
        if (!stored) log(Telemetry.line("heartbeat_not_persisted", source = logicalSource, count = 1))
        return stored
    }
}
