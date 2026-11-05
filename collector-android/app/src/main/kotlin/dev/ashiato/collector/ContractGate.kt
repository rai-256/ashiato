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
    /** 読める答えで、系統が自分と同じ。S-01 は版 A 以下を全部受け付ける */
    data class Version(val version: Int) : ContractAnswer

    /**
     * 読める答えだが、系統が自分と違う（ST54 / D7）。何も送らない。
     * 系統の比べは [readContractAnswer] の 1 か所だけで行い、送り手は比べ直さない（/code-review R36）
     */
    data class ChannelMismatch(val expected: Channel, val actual: Channel) : ContractAnswer

    /** 404（問い合わせの口が無い古い S-01）。D9 の（b） */
    data object NoEndpoint : ContractAnswer

    /** 200 で読めない本文（公衆の Wi-Fi の入口の画面など）。24 時間続くまでは更新を待つ状態にしない（D22） */
    data object Unreadable : ContractAnswer

    /** 到達できない・時間切れ・401・403・5xx。今どおり一時的な失敗（更新が原因ではない） */
    data object Transient : ContractAnswer
}

enum class Channel { Dev, Official }

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
fun readContractAnswer(
    outcome: Outcome,
    expectedChannel: Channel = buildLocalChannel()
): ContractAnswer = when (outcome) {
    is Outcome.Unreachable -> ContractAnswer.Transient
    is Outcome.Responded -> when (outcome.status) {
        HTTP_OK -> parseContractVersion(outcome.body)
            ?.let { versionAnswer(it, responseChannel(outcome.headers), expectedChannel) }
            ?: ContractAnswer.Unreadable
        HTTP_NOT_FOUND -> ContractAnswer.NoEndpoint
        else -> ContractAnswer.Transient
    }
}

/** 読めた版を、応答の系統が手元と同じときだけ答えにする（違えば系統の食い違い）。 */
private fun versionAnswer(version: Int, channel: Channel, expected: Channel): ContractAnswer =
    if (channel == expected) ContractAnswer.Version(version) else ContractAnswer.ChannelMismatch(expected, channel)

/** 応答の `Ashiato-Channel` の見出し（大文字小文字を問わない）。無いか `dev` 以外なら正式版 */
private fun responseChannel(headers: Map<String, String>): Channel {
    val value = headers.entries
        .firstOrNull { (key, _) -> key.equals("Ashiato-Channel", ignoreCase = true) }
        ?.value?.trim()?.lowercase()
    return if (value == "dev") Channel.Dev else Channel.Official
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

    var channelNotifiedAtMs: Long?
        get() = prefs.getLong(CHANNEL_NOTIFIED_AT, NONE).takeIf { it != NONE }
        set(value) = putOrRemove(CHANNEL_NOTIFIED_AT, value)

    var channelMismatch: Boolean
        get() = prefs.getBoolean(CHANNEL_MISMATCH, false)
        set(value) { prefs.edit().putBoolean(CHANNEL_MISMATCH, value).apply() }

    /** 画面が出す行の元。止まっていなければ null */
    var notice: ContractNotice?
        get() {
            val days = prefs.getLong(NOTICE_DAYS, NONE).takeIf { it != NONE } ?: return null
            return ContractNotice(
                prefs.getBoolean(NOTICE_ONLY_UNREADABLE, false),
                days.toInt(),
                prefs.getBoolean(NOTICE_CHANNEL_MISMATCH, false),
            )
        }
        set(value) {
            val edit = prefs.edit()
            if (value == null) {
                edit.remove(NOTICE_DAYS).remove(NOTICE_ONLY_UNREADABLE)
            } else {
                edit.putLong(NOTICE_DAYS, value.days.toLong())
                    .putBoolean(NOTICE_ONLY_UNREADABLE, value.onlyUnreadable)
                    .putBoolean(NOTICE_CHANNEL_MISMATCH, value.channelMismatch)
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
        private const val NOTICE_CHANNEL_MISMATCH = "notice_channel_mismatch"
        private const val CHANNEL_NOTIFIED_AT = "channel_notified_at_ms"
        private const val CHANNEL_MISMATCH = "channel_mismatch"
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
    val headline: String? get() = if (waiting) {
        contractHeadline(onlyUnreadable)
    } else {
        null
    }
}

/**
 * 「S-01 の更新を待っている」状態を 1 つの値で持つ（D9・D22・D24（仮））。
 * 理由が 1 つでもあれば待っている。**待っていても行は捨てない**（D17）。
 */
class ContractGate(
    private val store: ContractWaitStore,
    private val log: (String) -> Unit = {},
    val localChannel: Channel = buildLocalChannel(),
) {
    @Volatile
    var reasons: ContractReasons = ContractReasons()
        private set

    /**
     * 起動してから読める答えを 1 度でも得たか（理由はメモリにしか無いので、それまで「待っていない」は当てにならない）。
     * 404 と読めない答えでは立てない（PC の `reasons_known` と同じ。final review R3）
     */
    @Volatile
    var known: Boolean = false
        private set

    /** いまの理由で通知と画面の行を決めてよいか。待っているなら確か、待っていないは読める答えを得てからだけ確か（R3） */
    val decided: Boolean get() = known || reasons.waiting

    val waiting: Boolean get() = reasons.waiting

    val channelMismatch: Boolean get() = store.channelMismatch

    /**
     * 系統の違いをログに出したか（メモリだけ）。続く間は契機ごとに出さず、状態に入ったときに 1 回だけ出す
     * （PC の `channel_mismatch_logged` と同じ。/code-review R31）。起こし直した後は 1 回出す
     */
    @Volatile
    private var mismatchLogged = false

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
            is ContractAnswer.ChannelMismatch -> {
                known = true
                store.channelMismatch = true
                if (!mismatchLogged) {
                    log(
                        Telemetry.line(
                            "channel_mismatch",
                            source = null,
                            error = "channel_mismatch",
                            trigger = channelLabel(answer.expected),
                            availability = channelLabel(answer.actual)
                        )
                    )
                    mismatchLogged = true
                }
            }
            is ContractAnswer.Version -> {
                known = true
                store.channelMismatch = false
                mismatchLogged = false
                setUnreadableSince(null)
                val posted = outcomes.any { it.posted }
                // 原本の糸の印は、記録の送りが 426 だった契機でも**必ず読んで消す**（final review R7）——
                // `||` の右に置くと短絡で残り、次の契機に（c）が立ち直る
                val originals = originalsRefused.getAndSet(false)
                reasons = ContractReasons(
                    ahead = outcomes.any { it.ahead },
                    refused = (posted && outcomes.any { it.refused }) || originals,
                    // （d）は「版で断られた行がまだ未送信に残っている」。その回に送ったかに依らない（final review R2。
                    // 断られた行を飛ばして新しい行が受け付けられた回に抜けると、通知と出した時刻が消えて次の契機で出直す）
                    rejected = outcomes.any { it.rejectedByVersion },
                )
            }
            ContractAnswer.NoEndpoint -> {
                // A を得られない契機は理由を足すだけ。落とすのは読める答えの契機だけ（D9）。
                // **known は立てない**（final review R3）—— 再起動の直後は前の理由がメモリに無い。読める答えだけが理由を全部決める
                reasons = reasons.copy(noEndpoint = true)
            }
            ContractAnswer.Unreadable -> {
                // known は立てない（R3）。24 時間に満たない読めない答えで「待っていない」と決めると、出ていた通知と画面の行が消える
                // 数え始めが今より後（時計が先へ進んでいた間に数え始めた）なら今へ置き直す。置き直さないと時計が戻った後は
                // 何日経っても 24 時間に届かず、促されないまま 90 日で捨てる（code-verify R18。design D25（仮））。
                // 読める答え以外で数え始めを動かすのはここだけ
                val since = minOf(store.unreadableSinceMs ?: Long.MAX_VALUE, now.toEpochMilli())
                setUnreadableSince(since)
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
 * 7 日前の知らせへ添える案内。再起動の後は理由がメモリに無い（[ContractGate.decided] が偽）ので、
 * 確かめるまでは画面の行と同じ SharedPreferences の写しに頼る（D6・D24（仮））。
 */
fun contractHeadlineFor(gate: ContractGate, store: ContractWaitStore): String? =
    if (gate.decided) {
        withChannelMismatch(gate.reasons.headline, gate.channelMismatch, gate.localChannel)
    } else {
        withChannelMismatch(store.notice?.headline, store.channelMismatch, gate.localChannel)
    }

/** 案内に、系統の食い違いの案内を「。」で継ぎ足す（どちらか片方だけならそれだけ。両方無ければ null）。 */
private fun withChannelMismatch(headline: String?, mismatch: Boolean, local: Channel): String? {
    val channel = if (mismatch) channelMismatchHeadline(local) else null
    return listOfNotNull(headline, channel).takeIf { it.isNotEmpty() }?.joinToString("。")
}

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
        return readContractAnswer(probe(), gate.localChannel)
    }

    /** 契機の終わりに結果を取り込む。待っている状態を抜けたら真。 */
    fun settle(answer: ContractAnswer, outcomes: List<ContractOutcome>): Boolean = gate.observe(answer, now(), outcomes)
}

/** 自分の系統（D1）。`BuildConfig.CHANNEL` を読むのはここだけ。部品は引数で受け、既定にこれを使う */
fun buildLocalChannel(): Channel = if (BuildConfig.CHANNEL == "dev") Channel.Dev else Channel.Official

private fun channelLabel(channel: Channel): String = if (channel == Channel.Dev) "dev" else "official"
