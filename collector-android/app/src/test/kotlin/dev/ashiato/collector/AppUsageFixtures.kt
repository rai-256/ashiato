// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.io.File
import java.nio.file.Files
import java.time.Instant
import java.time.ZoneId

/**
 * Task 3 の試験の足場（取得の契機を 1 つずつ回す）。
 *
 * **時計は本物の [AgeClock] を通す** —— 単調な経過を偽の値で作ると、3.3 の
 * 「時計が飛んでいる間は取り直さない」が試験の中だけの算術になる。
 * [advance] は壁時計と単調時計を一緒に進め（飛んでいない）、
 * [jumpWall] は壁時計だけを飛ばす（端末の時刻が変わった）。
 */
class UsageTestEnv(
    val source: UsageSource,
    /** 取得時点の端末の地域（design D7）。契機のあいだに変えられる */
    var zone: ZoneId = ZoneId.of("Asia/Tokyo"),
    /** 取得時点に解決したアプリの表示名。契機のあいだに変えられる（アプリの更新・言語の変更） */
    var label: String? = "例のアプリ",
) {
    val dir: File = Files.createTempDirectory("st06-usage").toFile()
    val lines: MutableList<String> = mutableListOf()
    private val log: (String) -> Unit = { lines += it }
    val clock: FakeDeviceClock = FakeDeviceClock(wall = Instant.parse("2026-05-20T09:00:00Z").toEpochMilli())
    val age: AgeClock = AgeClock(clock, File(dir, "age-clock.txt"), log)
    val outbox: Outbox<IngestRequest> = testOutbox()
    private var ids = 0
    private var current: AppUsageSourceAdapter? = null

    /** いまの端末の壁時計。親が窓を組み立てるのに使う値そのもの */
    val now: Instant get() = Instant.ofEpochMilli(clock.wall)

    /** 保存された窓の終わり（試験が直接読む）。まだ 1 度も取れていなければ null */
    fun savedEnd(): Instant? = newStore().load()?.end

    private fun newStore() =
        UsageWindowStore(usageWindowFile(dir, APP_USAGE_LOGICAL_SOURCE), APP_USAGE_LOGICAL_SOURCE, log)

    /** いま走っているソース。[restart] を挟むまで同じもの */
    fun adapter(): AppUsageSourceAdapter = current ?: AppUsageSourceAdapter(
        source = source,
        outbox = outbox,
        windowStore = newStore(),
        labels = { label },
        userId = { "user-1" },
        deviceId = "device-1",
        zone = { zone },
        age = age::now,
        newId = { "r${ids++}" },
        capabilityOf = { Capability.of(permission = true, sensor = true, network = true) },
        log = log,
    ).also { current = it }

    /** 収集が止まって、また始まる（プロセスが作り直される）。 */
    fun restart() {
        current?.stop()
        current = null
    }

    /**
     * 取得の契機が 1 回来た。**親（`LocationService`）が渡す窓と同じ形**で呼ぶ ——
     * 位置は `CollectionWindow(now - intervalMs, now)` を渡している。
     */
    fun collect(): CollectionResult =
        adapter().collect(CollectionWindow(now.minusMillis(USAGE_INTERVAL_MS), now))

    /** 眠らずに `ms` 経った（壁時計と単調時計が一緒に進む＝時計は飛んでいない）。 */
    fun advance(ms: Long) = clock.advance(ms)

    /** 端末の時刻だけが `ms` 飛んだ（単調な経過はそのまま）。 */
    fun jumpWall(ms: Long) {
        clock.wall += ms
    }

    /** 未送信に積まれた記録。 */
    fun records(): List<IngestRequest> = outbox.snapshot()
}

/** 記録の `payload` の 1 欄を読む（無ければ null）。 */
fun IngestRequest.payloadText(key: String): String? =
    (payload[key] as? kotlinx.serialization.json.JsonPrimitive)?.content

/** 記録の `raw` を読み解いた 1 欄（無ければ null）。 */
fun IngestRequest.rawText(key: String): String? =
    ((ingestJson.parseToJsonElement(raw) as kotlinx.serialization.json.JsonObject)[key]
        as? kotlinx.serialization.json.JsonPrimitive)?.content

/** 取得元が返す 1 件（欄を全部埋めた形）。 */
fun fullUsageEvent(
    at: String,
    configuration: android.content.res.Configuration? = null,
): UsageEventSnapshot = UsageEventSnapshot(
    packageName = "dev.ashiato.example",
    className = "dev.ashiato.example.MainActivity",
    eventType = 7,
    at = Instant.parse(at),
    configuration = configuration,
    shortcutId = "shortcut-1",
    interactionAction = "android.intent.action.VIEW",
    interactionCategory = "android.intent.category.DEFAULT",
    standbyBucket = 20,
)
