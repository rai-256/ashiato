// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.io.File
import java.nio.file.Files

/**
 * 置き場を関心にしない試験のための Outbox。**毎回まっさらなディレクトリを使う。**
 *
 * `Outbox` に「メモリだけの既定」を持たせない代わりにここへ置く ——
 * 既定があると、本番でうっかりそれが選ばれて未送信が無言で消える
 * （深掘り 第 2 回で実際に起きていた欠陥）。
 */
fun testOutbox(): Outbox<IngestRequest> =
    Outbox.inDir(Files.createTempDirectory("outbox").toFile(), IngestRequest.serializer()) {}

/** 生存信号ぶん。**記録と同じ仕組み**（型が違うだけ）。 */
fun testHeartbeatOutbox(): Outbox<HeartbeatRequest> =
    Outbox.inDir(Files.createTempDirectory("heartbeat").toFile(), HeartbeatRequest.serializer()) {}

/**
 * 試験で時刻を進められる端末の時計（ST04）。壁時計・単調時計・起動回数を別々に動かせる ——
 * **時計の飛びと再起動**を作るため（design D2）。
 */
class FakeDeviceClock(
    var wall: Long = 1_757_000_000_000L,
    var mono: Long = 1_000L,
    var boot: Int? = 1,
) : DeviceClock {
    override fun wallMs(): Long = wall

    override fun monoMs(): Long = mono

    override fun bootCount(): Int? = boot

    /** 眠らずに `ms` 経った（壁時計と単調時計が一緒に進む）。 */
    fun advance(ms: Long) {
        wall += ms
        mono += ms
    }

    /** 再起動した。単調時計は小さな値から始まり、起動回数が 1 つ増える。止まっていた間に壁時計が `wallGapMs` 進む。 */
    fun reboot(wallGapMs: Long) {
        wall += wallGapMs
        mono = 500L
        boot = boot?.plus(1)
    }
}

/** 区切りの置き場と時計を 1 つのディレクトリに組む（ST04 の試験の足場）。 */
class TestStores(
    val dir: File = Files.createTempDirectory("st04").toFile(),
    val clock: FakeDeviceClock = FakeDeviceClock(),
    val lines: MutableList<String> = mutableListOf(),
    segmentBytes: Long = SegmentStore.SEGMENT_BYTES,
) {
    val log: (String) -> Unit = { lines += it }
    val age = AgeClock(clock, File(dir, "age-clock.txt"), log)
    val unreadable = File(dir, "unreadable.jsonl")
    var unreadableSeen = 0
    val recordsDir = File(dir, "records")
    val recordStore = SegmentStore(recordsDir, IngestRequest.serializer(), unreadable, log, segmentBytes) { unreadableSeen += it }
    val records = Outbox(recordStore, age::now)
    val beats = Outbox(
        SegmentStore(File(dir, "heartbeats"), HeartbeatRequest.serializer(), unreadable, log),
        age::now,
    )
    val drops = Outbox(SegmentStore(File(dir, "drops"), DropReport.serializer(), unreadable, log), age::now)
    private var nextId = 0
    var now: java.time.Instant = java.time.Instant.parse("2026-09-14T00:00:00Z")
    val ledger = DropLedger(File(dir, "drops-open.json"), drops, { "user-1" }, "device-1", { now }, { "r${nextId++}" }, log)
}
