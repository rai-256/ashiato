// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.content.SharedPreferences
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.int
import kotlinx.serialization.json.jsonObject
import java.time.Duration
import java.time.Instant
import java.util.concurrent.atomic.AtomicBoolean

/**
 * `GET /contract` の答え（ST50 / design D9・D11・D18（仮））。送る契機の頭で 1 回引き、その契機の 5 つの送り手に同じ答えを渡す。
 * **版 A を得たときだけ**送る。ほかは何も送らずに残す。
 */
sealed interface ContractAnswer {
    /** 読める答え。S-01 は版 A 以下を全部受け付ける */
    data class Version(val version: Int) : ContractAnswer

    /** 404（問い合わせの口が無い古い S-01）。D9 の（b） */
    data object NoEndpoint : ContractAnswer

    /** 200 で読めない本文（公衆の Wi-Fi の入口の画面など）。24 時間続くまでは更新を待つ状態にしない（D22） */
    data object Unreadable : ContractAnswer

    /** 到達できない・時間切れ・401・403・5xx。今どおり一時的な失敗（更新が原因ではない） */
    data object Transient : ContractAnswer
}

/** 1 回の送りの結果のうち、S-01 の更新を待つ理由（D9 の（a）〜（d））に要るもの。 */
data class ContractOutcome(
    /** 版 > A の行が未送信にある */
    val ahead: Boolean = false,
    /** 要求を載せて送った */
    val posted: Boolean = false,
    /** 送った要求が 426 で断られた */
    val refused: Boolean = false,
    /** 送った要求の 1 件以上が `unknown_field` か `contract_unsupported` で断られた */
    val rejectedByVersion: Boolean = false,
) {
    operator fun plus(other: ContractOutcome) = ContractOutcome(
        ahead = ahead || other.ahead,
        posted = posted || other.posted,
        refused = refused || other.refused,
        rejectedByVersion = rejectedByVersion || other.rejectedByVersion,
    )
}

/** 問い合わせの口の答えを読む（200 の本文が `{"contract_version": <正の整数>}`）。 */
fun readContractAnswer(outcome: Outcome): ContractAnswer = when (outcome) {
    is Outcome.Unreachable -> ContractAnswer.Transient
    is Outcome.Responded -> when (outcome.status) {
        HTTP_OK -> parseContractVersion(outcome.body)?.let { ContractAnswer.Version(it) } ?: ContractAnswer.Unreadable
        HTTP_NOT_FOUND -> ContractAnswer.NoEndpoint
        else -> ContractAnswer.Transient
    }
}

private const val HTTP_OK = 200
private const val HTTP_NOT_FOUND = 404

private fun parseContractVersion(body: String): Int? = runCatching {
    val field = Json.parseToJsonElement(body).jsonObject["contract_version"] as? JsonPrimitive
    // 数字だけ。文字列の "1"・小数・負・0 は読めない答え
    field?.takeIf { !it.isString }?.int?.takeIf { it >= 1 }
}.getOrNull()

/** 200 の読めない答えがこれだけ続いたら、更新を待つ状態に入る（D22）。 */
val UNREADABLE_LIMIT: Duration = Duration.ofHours(UNREADABLE_LIMIT_HOURS)

private const val UNREADABLE_LIMIT_HOURS = 24L

/**
 * 手元に持つもの。**収集アプリや端末の再起動をまたぐ**（`SharedPreferences`。FR-126 の★）。
 * 通知を前に出した時刻と、画面が読む「いま止まっている」の写し（D6）も同じ置き場に持つ。
 */
class ContractWaitStore(private val prefs: SharedPreferences) {
    /** 200 の読めない答えを最初に得た時刻（エポックミリ秒）。無ければ null */
    var unreadableSinceMs: Long?
        get() = prefs.getLong(UNREADABLE_SINCE, NONE).takeIf { it != NONE }
        set(value) = putOrRemove(UNREADABLE_SINCE, value)

    /** 更新を促す通知を前に出した時刻（エポックミリ秒）。無ければ null（止まっていない・まだ出していない） */
    var notifiedAtMs: Long?
        get() = prefs.getLong(NOTIFIED_AT, NONE).takeIf { it != NONE }
        set(value) = putOrRemove(NOTIFIED_AT, value)

    /** 画面が出す行の元。止まっていなければ null */
    var notice: ContractNotice?
        get() {
            val days = prefs.getLong(NOTICE_DAYS, NONE).takeIf { it != NONE } ?: return null
            return ContractNotice(prefs.getBoolean(NOTICE_ONLY_UNREADABLE, false), days.toInt())
        }
        set(value) {
            val edit = prefs.edit()
            if (value == null) {
                edit.remove(NOTICE_DAYS).remove(NOTICE_ONLY_UNREADABLE)
            } else {
                edit.putLong(NOTICE_DAYS, value.days.toLong()).putBoolean(NOTICE_ONLY_UNREADABLE, value.onlyUnreadable)
            }
            edit.apply()
        }

    private fun putOrRemove(key: String, value: Long?) {
        val edit = prefs.edit()
        if (value == null) edit.remove(key) else edit.putLong(key, value)
        edit.apply()
    }

    companion object {
        const val PREFS = "contract-wait"
        private const val UNREADABLE_SINCE = "unreadable_since_ms"
        private const val NOTIFIED_AT = "notified_at_ms"
        private const val NOTICE_DAYS = "notice_days"
        private const val NOTICE_ONLY_UNREADABLE = "notice_only_unreadable"
        private const val NONE = Long.MIN_VALUE
    }
}

/** 待っている理由（D9 の（a）〜（e））。 */
data class ContractReasons(
    val ahead: Boolean = false,
    val noEndpoint: Boolean = false,
    val refused: Boolean = false,
    val rejected: Boolean = false,
    val unreadableLong: Boolean = false,
) {
    val waiting: Boolean get() = ahead || noEndpoint || refused || rejected || unreadableLong

    /** 理由が（e）だけか（知らせの文言を分ける。D22） */
    val onlyUnreadable: Boolean get() = unreadableLong && !(ahead || noEndpoint || refused || rejected)

    /** 待っているときの案内（待っていなければ null。画面・通知・7 日前の知らせが使う。D6） */
    val headline: String? get() = if (waiting) contractHeadline(onlyUnreadable) else null
}

/**
 * 「S-01 の更新を待っている」状態を 1 つの値で持つ（D9・D22・D24（仮））。
 * 理由が 1 つでもあれば待っている。**待っていても行は捨てない**（D17）。
 */
class ContractGate(private val store: ContractWaitStore, private val log: (String) -> Unit = {}) {
    @Volatile
    var reasons: ContractReasons = ContractReasons()
        private set

    /** 起動してから読める答えか 404 か読めない答えを 1 度でも得たか（理由はメモリにしか無いので、それまで「待っていない」は当てにならない） */
    @Volatile
    var known: Boolean = false
        private set

    val waiting: Boolean get() = reasons.waiting

    /** 原本の送り手が 426 で断られた（非同期。読める答えの契機の [observe] が読んで消す） */
    private val originalsRefused = AtomicBoolean(false)

    /** 原本の送り手が 426 で断られた。 */
    fun noteOriginalsRefused() {
        originalsRefused.set(true)
    }

    /**
     * 1 つの契機の結果を取り込む。**待っている状態を抜けたら真**（呼び手は「飛ばす」印を捨てる。D14）。
     * 一時的な失敗の契機は何も変えない（更新が原因ではないので、始めも終わりもしない）。
     */
    fun observe(answer: ContractAnswer, now: Instant, outcomes: List<ContractOutcome>): Boolean {
        val was = reasons.waiting
        when (answer) {
            ContractAnswer.Transient -> return false
            is ContractAnswer.Version -> {
                known = true
                setUnreadableSince(null)
                val posted = outcomes.any { it.posted }
                reasons = ContractReasons(
                    ahead = outcomes.any { it.ahead },
                    refused = (posted && outcomes.any { it.refused }) || originalsRefused.getAndSet(false),
                    rejected = posted && outcomes.any { it.rejectedByVersion },
                )
            }
            ContractAnswer.NoEndpoint -> {
                // A を得られない契機は理由を足すだけ。落とすのは読める答えの契機だけ（D9）
                known = true
                reasons = reasons.copy(noEndpoint = true)
            }
            ContractAnswer.Unreadable -> {
                known = true
                val since = store.unreadableSinceMs ?: now.toEpochMilli().also { setUnreadableSince(it) }
                val long = now.toEpochMilli() - since >= UNREADABLE_LIMIT.toMillis()
                reasons = reasons.copy(unreadableLong = long)
            }
        }
        return was && !reasons.waiting
    }

    private fun setUnreadableSince(value: Long?) {
        if (store.unreadableSinceMs == value) return
        runCatching { store.unreadableSinceMs = value }.onFailure {
            log(Telemetry.line("contract_wait_save_failed", source = null, error = it.javaClass.simpleName))
        }
    }
}

/**
 * 7 日前の知らせへ添える案内。再起動の後は理由がメモリに無い（[ContractGate.known] が偽）ので、
 * 確かめるまでは画面の行と同じ SharedPreferences の写しに頼る（D6・D24（仮））。
 */
fun contractHeadlineFor(gate: ContractGate, store: ContractWaitStore): String? =
    if (gate.known) gate.reasons.headline else store.notice?.headline

/**
 * 送る契機の頭の確かめと、その結果を状態へ取り込む係（D18（仮）。答えを覚えない）。
 * [probe] は `GET /contract` を打つ口（試験は偽物に差し替える）。
 */
class ContractGuard(
    private val probe: () -> Outcome,
    val gate: ContractGate,
    private val now: () -> Instant = { Instant.now() },
) {
    /** 契機の頭で 1 回引く。 */
    fun ask(): ContractAnswer {
        return readContractAnswer(probe())
    }

    /** 契機の終わりに結果を取り込む。待っている状態を抜けたら真。 */
    fun settle(answer: ContractAnswer, outcomes: List<ContractOutcome>): Boolean = gate.observe(answer, now(), outcomes)
}
