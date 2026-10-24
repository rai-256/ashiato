// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.io.IOException
import java.io.InputStream
import java.security.MessageDigest
import java.time.Duration
import java.time.Instant
import java.util.concurrent.atomic.AtomicBoolean

private const val RETRY_FIRST_MINUTES = 1L
private const val RETRY_MAX_HOURS = 1L
private const val GIVE_UP_DAYS = 7L

/** 試し直しの間隔の最初（design D9。倍にして [UPLOAD_RETRY_MAX] で止める）。 */
val UPLOAD_RETRY_FIRST: Duration = Duration.ofMinutes(RETRY_FIRST_MINUTES)

val UPLOAD_RETRY_MAX: Duration = Duration.ofHours(RETRY_MAX_HOURS)

/** 最初の `409 record_missing` からこれだけ続いたら、原本の欠け `record_missing` にする（design D9（仮）/ R7）。 */
val RECORD_MISSING_GIVE_UP: Duration = Duration.ofDays(GIVE_UP_DAYS)

/**
 * 原本の本文の上限（design D10（仮）。S-01 の `ORIGINAL_BODY_LIMIT` と同じ 1 GiB）。
 * 超える原本は送らずに欠け `too_large` にする —— S-01 は長さを見て本文を読む前に `413` で閉じ、固定長の流し送りは
 * 書き込みの途中の `IOException`（届かない）に畳まれるので、端末が先に決めないと 1 時間ごとに読み直し続ける（R4）。
 */
const val ORIGINAL_BODY_LIMIT: Long = 1L shl 30

/** 原本の受け口（`HEAD` / `PUT /media/originals/{sha256}`）。試験では偽物に差し替える。 */
interface OriginalTransport {
    /** 受け取り済みか。`200` なら済み、`404` なら未 */
    fun head(sha256: String): Outcome

    /** 本文は [body] のバイトのまま、[length] バイトの固定長で流す */
    fun put(sha256: String, length: Long, body: InputStream): Outcome
}

/** 端末のファイルを開いた結果。 */
sealed interface OriginalOpen {
    class Opened(val input: InputStream) : OriginalOpen

    /** 項目が無い（端末から消えた） */
    data object Vanished : OriginalOpen

    /** 項目はあるが開けない */
    data object Unreadable : OriginalOpen

    /** 権限がいまは足りない（「選んだ写真だけ」から外れた など）。待てば戻りうるので欠けにしない */
    data object Denied : OriginalOpen
}

/** 原本を開く口。**位置の権限が無い間は呼ばない**（Q15。呼び手 [OriginalUploader] が門を持つ）。 */
interface OriginalSource {
    fun open(entry: OriginalEntry): OriginalOpen
}

private class Backoff(val failures: Int, val nextAtMs: Long)

/** 送り手が端末について聞く口（権限・通信・時計）。 */
class OriginalEnvironment(
    /** 写真の位置の権限が無ければ [MediaAccess.NO_LOCATION]（Q15）。それ以外の拒否も開かない */
    val access: () -> MediaAccess,
    /** いまの通信が従量制でないか（`NET_CAPABILITY_NOT_METERED`） */
    val unmetered: () -> Boolean,
    val now: () -> Instant,
    /** 送る本文の上限（[ORIGINAL_BODY_LIMIT]。試験だけが小さくする） */
    val bodyLimit: Long = ORIGINAL_BODY_LIMIT,
)

/** 1 行の送りの結果。 */
private sealed interface Step {
    data object Sent : Step

    /** 行を消した（受け取り済み・欠けとして報告した） */
    data object Settled : Step

    /** 試し直す。[stop] なら今回はここで止める（届かない・置き場に書けない） */
    class Retry(val kind: String, val stop: Boolean) : Step
}

/** 1 回目の読みの結果。 */
private sealed interface First {
    /** 記録のハッシュと合った。[length] は読んだバイト数（`PUT` の固定長） */
    class Matched(val length: Long) : First

    class Stop(val step: Step) : First
}

/**
 * 原本の送り手（design D9）。**1 つずつ**、`ready` の行を送る。
 *
 * - 順: `backlog = false` を `queued_at` の古い順、次に `backlog = true` を `date_added` の新しい順
 * - 通信: `backlog = false` はどの通信でも、`backlog = true` は従量制でない通信のときだけ
 * - 送る前に `HEAD` で問い合わせる。受け取り済みなら**ファイルを読まずに**行を消す
 * - 2 回読む: 1 回目でハッシュを確かめ（記録のハッシュと違えば `changed`）、合えば 2 回目で読みながら `PUT`
 * - 応答ごとの扱いと試し直しの間隔は [put] と [headStep]。動画は行を持たないので送らない
 *
 * **写しは作らない**（Q4）。**位置の権限が無い間は何も開かない**（Q15）。
 * 1 回の [run] は送れるものが尽きるか、届かなくなるまで続く。同時に 2 本は走らない。
 */
class OriginalUploader(
    private val ledger: OriginalLedger,
    source: OriginalSource,
    transport: OriginalTransport,
    reporter: OriginalMissingReporter,
    private val env: OriginalEnvironment,
    private val log: (String) -> Unit = {},
) {
    private val exchange = OriginalExchange(ledger, source, transport, reporter, env.now, env.bodyLimit)
    private val running = AtomicBoolean(false)
    private val backoff = HashMap<String, Backoff>()

    /** いま [run] が走っている（呼び手が重ねて起こさないための目安） */
    val busy: Boolean get() = running.get()

    /** 送れるものを送る。送れた件数を返す。 */
    fun run(): Int {
        if (!running.compareAndSet(false, true)) return 0
        try {
            return if (mayOpenFiles()) pass() else 0
        } finally {
            running.set(false)
        }
    }

    /** 位置の権限が無い間は、写真のファイルを開かない（Q15。開くと位置が 0.0 に塗られる） */
    private fun mayOpenFiles(): Boolean = when (env.access()) {
        MediaAccess.NONE, MediaAccess.NO_LOCATION -> false
        MediaAccess.FULL, MediaAccess.PARTIAL -> true
    }

    private fun pass(): Int {
        var sent = 0
        val tried = HashSet<String>()
        val failures = LinkedHashMap<String, Int>()
        var stopped = false
        while (!stopped) {
            val row = next(tried)
            if (row == null) {
                stopped = true
            } else {
                tried += row.sha256
                when (val step = attempt(row)) {
                    Step.Sent -> sent++
                    Step.Settled -> Unit
                    is Step.Retry -> {
                        failures.merge(step.kind, 1, Int::plus)
                        scheduleRetry(row)
                        stopped = step.stop
                    }
                }
            }
        }
        if (sent > 0) log(Telemetry.line("original_sent", source = null, count = sent))
        // 種別と件数だけ（ファイル名・位置・保存先・ハッシュは出さない。D21）
        failures.forEach { (kind, count) ->
            log(Telemetry.line("original_send_failed", source = null, count = count, error = kind))
        }
        return sent
    }

    private fun attempt(row: OriginalEntry): Step = try {
        exchange.send(row)
    } catch (_: IOException) {
        // 読んでいる途中で切れた。待てば戻りうるので欠けにしない
        Step.Retry("read_failed", stop = false)
    }

    private fun scheduleRetry(row: OriginalEntry) {
        val failed = (backoff[row.sha256]?.failures ?: 0) + 1
        backoff[row.sha256] = Backoff(failed, env.now().toEpochMilli() + intervalMs(failed))
    }

    /** 次に送る行。バックオフ中・今回すでに当たった行・従量制で送れない過去ぶんは飛ばす。 */
    private fun next(tried: Set<String>): OriginalEntry? {
        val at = env.now().toEpochMilli()
        val metered = !env.unmetered()
        return ledger.ready()
            .asSequence()
            .filter { it.sha256 !in tried }
            .filter { !(it.backlog && metered) }
            .filter { (backoff[it.sha256]?.nextAtMs ?: Long.MIN_VALUE) <= at }
            .minWithOrNull(ORDER)
    }

    private fun intervalMs(failures: Int): Long {
        val doubling = UPLOAD_RETRY_FIRST.toMillis() shl (failures - 1).coerceAtMost(MAX_DOUBLINGS)
        return doubling.coerceAtMost(UPLOAD_RETRY_MAX.toMillis())
    }

    private companion object {
        const val MAX_DOUBLINGS = 20

        /** 新しいもの（`backlog = false`）を `queued_at` の古い順、次に過去ぶんを `date_added` の新しい順 */
        val ORDER: Comparator<OriginalEntry> = Comparator { a, b ->
            when {
                a.backlog != b.backlog -> if (a.backlog) 1 else -1
                !a.backlog -> compareValuesBy(a, b, { it.queuedAt }, { it.sha256 })
                else -> compareValues(b.dateAdded, a.dateAdded).takeIf { it != 0 } ?: a.sha256.compareTo(b.sha256)
            }
        }
    }
}

/**
 * 1 行ぶんの送り（`HEAD` → 1 回目の読み → 2 回目の読みで `PUT`）と、応答ごとの扱い（design D9）。
 * 試し直しの間隔と順は持たない（[OriginalUploader]）。
 */
private class OriginalExchange(
    private val ledger: OriginalLedger,
    private val source: OriginalSource,
    private val transport: OriginalTransport,
    private val reporter: OriginalMissingReporter,
    private val now: () -> Instant,
    private val bodyLimit: Long,
) {
    fun send(row: OriginalEntry): Step {
        headStep(row)?.let { return it }
        return when (val first = hashFirst(row)) {
            is First.Stop -> first.step
            is First.Matched -> put(row, first.length)
        }
    }

    /** `HEAD`。続けてよければ null。 */
    private fun headStep(row: OriginalEntry): Step? = when (val outcome = transport.head(row.sha256)) {
        is Outcome.Unreachable -> Step.Retry("unreachable", stop = true)
        is Outcome.Responded -> when (outcome.status) {
            // 受け取り済み: 読まずに行を消す（入れ直した後に読み直して送り直さない。R9）
            HTTP_OK -> removed(row, Step.Settled)
            HTTP_NOT_FOUND -> null
            else -> Step.Retry("server_${outcome.status}", stop = true)
        }
    }

    /** 1 回目の読み。ハッシュを確かめて、読んだバイト数を返す。上限を超えたらそこで読むのをやめる。 */
    private fun hashFirst(row: OriginalEntry): First {
        // 一覧の大きさが上限を超えていれば開かない（読み直しても受け取られない）
        if (row.size > bodyLimit) return First.Stop(settle(row, OriginalMissingReason.TOO_LARGE))
        return when (val opened = source.open(row)) {
            is OriginalOpen.Opened -> firstRead(row, opened)
            else -> First.Stop(unopened(row, opened))
        }
    }

    private fun firstRead(row: OriginalEntry, opened: OriginalOpen.Opened): First {
        val (sha, length) = opened.input.use { hashAndCount(it, bodyLimit) }
        return when {
            // 一覧の大きさより実物が大きかった。`PUT` しても受け取られない
            length > bodyLimit -> First.Stop(settle(row, OriginalMissingReason.TOO_LARGE))
            sha == row.sha256 -> First.Matched(length)
            // 中身が変わった: 変わる前の記録の原本として送らない。新しい中身は読み手が次の記録にする
            else -> First.Stop(settle(row, OriginalMissingReason.CHANGED))
        }
    }

    /** 開けなかったときの扱い（1 回目も 2 回目も同じ）。 */
    private fun unopened(row: OriginalEntry, opened: OriginalOpen): Step = when (opened) {
        OriginalOpen.Vanished -> settle(row, OriginalMissingReason.VANISHED)
        OriginalOpen.Unreadable -> settle(row, OriginalMissingReason.UNREADABLE)
        OriginalOpen.Denied -> Step.Retry("denied", stop = false)
        is OriginalOpen.Opened -> error("opened")
    }

    /** 欠けを報告して行を消す。 */
    private fun settle(row: OriginalEntry, reason: OriginalMissingReason): Step {
        reporter.report(row, reason)
        return removed(row, Step.Settled)
    }

    /** 行を消す。消せなければ行を残して止める（報告だけが先に出ても、次の回は同じ報告を重ねるだけで冪等） */
    private fun removed(row: OriginalEntry, done: Step): Step =
        if (ledger.removeAll(listOf(row.sha256))) done else Step.Retry("ledger_write_failed", stop = true)

    /** 2 回目の読みで流して `PUT` する。 */
    private fun put(row: OriginalEntry, length: Long): Step = when (val opened = source.open(row)) {
        is OriginalOpen.Opened -> when (val outcome = opened.input.use { transport.put(row.sha256, length, it) }) {
            is Outcome.Unreachable -> Step.Retry("unreachable", stop = true)
            is Outcome.Responded -> putResult(row, outcome.status, outcome.body)
        }
        else -> unopened(row, opened)
    }

    private fun putResult(row: OriginalEntry, status: Int, body: String): Step = when (status) {
        HTTP_OK, HTTP_CREATED -> removed(row, Step.Sent)
        HTTP_CONFLICT -> recordMissing(row)
        // 送っている途中で変わった（次の回の 1 回目で `changed` になる）か、S-01 が名前を決められなかった
        // （置き場の障害ではない）。どちらも欠けにせず行を残し、ほかの行は止めない
        HTTP_UNPROCESSABLE -> {
            val kind = if (body.contains(NAME_REFUSED)) NAME_REFUSED else "hash_mismatch"
            Step.Retry(kind, stop = false)
        }
        HTTP_TOO_LARGE -> settle(row, OriginalMissingReason.TOO_LARGE)
        // 503 storage_unavailable（D-02 に書けない）・そのほか: 行を残して試し直す
        else -> Step.Retry("server_$status", stop = true)
    }

    /** `409 record_missing`: 同じ間隔で試し直し、最初の 409 から 7 日続いたら欠けにする。 */
    private fun recordMissing(row: OriginalEntry): Step {
        val at = now().toEpochMilli()
        val since = row.missingSince ?: at.also { ledger.markMissingSince(row.sha256, it) }
        return if (at - since >= RECORD_MISSING_GIVE_UP.toMillis()) {
            settle(row, OriginalMissingReason.RECORD_MISSING)
        } else {
            Step.Retry("record_missing", stop = false)
        }
    }

    private companion object {
        const val HTTP_OK = 200
        const val HTTP_CREATED = 201
        const val HTTP_NOT_FOUND = 404
        const val HTTP_CONFLICT = 409
        const val HTTP_UNPROCESSABLE = 422
        const val HTTP_TOO_LARGE = 413

        /** `422` の本文の語（`crates/server` の `media.rs`） */
        const val NAME_REFUSED = "name_refused"
    }
}

private const val HASH_BUFFER_BYTES = 64 * 1024

/**
 * 流して読み、内容ハッシュ（16 進の小文字）と読んだバイト数を返す。メモリに全体を載せない。
 * [limit] を超えたら読むのをやめる（返す長さが [limit] より大きければ、ハッシュは途中までのもの）。
 */
private fun hashAndCount(input: InputStream, limit: Long): Pair<String, Long> {
    val digest = MessageDigest.getInstance("SHA-256")
    val buffer = ByteArray(HASH_BUFFER_BYTES)
    var length = 0L
    while (length <= limit) {
        val n = input.read(buffer)
        if (n < 0) break
        digest.update(buffer, 0, n)
        length += n
    }
    return digest.digest().joinToString("") { "%02x".format(it) } to length
}
