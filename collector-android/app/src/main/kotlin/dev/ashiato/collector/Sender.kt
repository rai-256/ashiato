// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import kotlinx.serialization.KSerializer
import kotlinx.serialization.builtins.ListSerializer

/** 取り込み口を叩いた結果。網の失敗と、サーバが返した応答を区別する。 */
sealed interface Outcome {
    /** 到達できなかった。**種別だけを持つ** —— 例外の文言は本文を含むことがある */
    data class Unreachable(val kind: String) : Outcome

    data class Responded(val status: Int, val body: String) : Outcome
}

/** 取り込み口への 1 回の POST。試験では偽物に差し替える。 */
fun interface Transport {
    fun post(bodyJson: String): Outcome
}

/**
 * 未送信をまとめて送る（design D9）。
 *
 * **成功した分だけを未送信から取り除く。** 全部やり直すと、1 件の恒久的な失敗が
 * 後続を永久に止める。取り込み口は冪等なので、成功したものを再送しても行は増えない（FR-22）。
 */
class Sender<T : Outboxable>(
    private val outbox: Outbox<T>,
    private val transport: Transport,
    private val serializer: KSerializer<T>,
    private val log: (String) -> Unit = {},
) {
    /** 送った件数と受け付けられた件数。 */
    data class Flushed(val sent: Int, val accepted: Int)

    fun flush(): Flushed {
        // **1 回に載せる件数を切る**（design D23）。切らないと、長い圏外のあと
        // 1 回の POST が読み取り上限を超え、1 件も取り除けないまま永久に繰り返す
        val batch = outbox.snapshot().take(MAX_BATCH)
        if (batch.isEmpty()) return Flushed(0, 0)

        val body = ingestJson.encodeToString(ListSerializer(serializer), batch)
        val accepted = when (val outcome = transport.post(body)) {
            is Outcome.Unreachable -> {
                // 未送信はそのまま残す。次の契機で再び送る（FR-10）
                log(Telemetry.line("send_failed", count = batch.size, error = outcome.kind))
                return Flushed(batch.size, 0)
            }

            is Outcome.Responded -> acceptedIds(batch, outcome)
        }

        if (!outbox.remove(accepted)) {
            // 取り除けたが置き場へ書けなかった。**次の起動で再送になる**（重複は入らない）
            log(Telemetry.line("outbox_shrink_failed", count = accepted.size))
        }
        log(Telemetry.line("send", count = batch.size))
        log(Telemetry.line("accepted", count = accepted.size))
        return Flushed(batch.size, accepted.size)
    }

    /**
     * 受け付けられた分の識別子。**結果は送った順に並ぶ**ので位置で対応づける
     * （docs/collector-contract.md §返る形）。
     *
     * **400 でも本文を読む。** 一部だけが不正なときに成功分を取り除けないと、
     * その 1 件が後続を永久に止める。
     */
    private fun acceptedIds(batch: List<T>, res: Outcome.Responded): List<String> {
        if (res.status == 401) {
            log(Telemetry.line("send_failed", count = batch.size, error = "unauthorized"))
            return emptyList()
        }
        // 5xx は本文が結果の配列でないことがある（サーバ側の失敗）。
        // **状態符号を落とさない** —— 落とすと 500 も 503 も本文欠落も同じ 1 行に潰れる
        if (res.status >= 500) {
            log(Telemetry.line("send_failed", count = batch.size, error = "server_${res.status}"))
            return emptyList()
        }
        val results = runCatching {
            ingestJson.decodeFromString<List<IngestResult>>(res.body)
        }.getOrElse {
            // 応答の形が読めないときは**何も取り除かない**。取り除くと記録が消える
            log(Telemetry.line("send_failed", count = batch.size, error = "unreadable_response"))
            return emptyList()
        }
        if (results.size != batch.size) {
            log(Telemetry.line("send_failed", count = batch.size, error = "result_count_mismatch"))
            return emptyList()
        }
        // **断られた分を黙って積み直さない**（review HIGH-12）。恒久的に断られる記録は
        // 未送信に居座り、5 分ごとに送られ続ける。理由の種別は私的データではないので出せる
        results.filter { !it.accepted }
            .groupingBy { it.error ?: "unknown" }
            .eachCount()
            .forEach { (kind, count) -> log(Telemetry.line("rejected", count = count, error = kind)) }
        return batch.filterIndexed { i, _ -> results[i].accepted }.map { it.id }
    }
}
