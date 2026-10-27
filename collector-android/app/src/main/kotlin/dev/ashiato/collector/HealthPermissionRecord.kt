// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.content.SharedPreferences
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import java.time.Instant
import java.time.ZoneId

/** 健康データの権限の状態を記録する論理ソース（design D1）。登録簿に同じ名前が要る。 */
const val HEALTH_LOGICAL_SOURCE: String = "c01-health-permission"

/** 観た契機（design D9（仮））。 */
object HealthTrigger {
    const val APP_OPEN = "app_open"
    const val REQUEST_RESULT = "request_result"
    const val PERIODIC = "periodic"
}

/**
 * 観た状態のうち、**変わったかの判定に使う 4 欄**（design D2）。時刻・経過・起動の識別は持たない。
 * [granted] は [HEALTH_PERMISSIONS] の鍵の順。使えないとき（`available` でない）は [features] も [granted] も全部 `false`。
 */
data class HealthState(
    val availability: HealthAvailability,
    val features: HealthFeatures,
    val granted: Map<String, Boolean>,
    val autoRequested: Boolean,
) {
    internal fun fields(): LinkedHashMap<String, kotlinx.serialization.json.JsonElement> = linkedMapOf(
        "availability" to JsonPrimitive(availability.wire),
        "features" to JsonObject(
            linkedMapOf(
                "history" to JsonPrimitive(features.history),
                "background" to JsonPrimitive(features.background),
            ),
        ),
        "granted" to JsonObject(granted.mapValuesTo(linkedMapOf()) { JsonPrimitive(it.value) }),
        "auto_requested" to JsonPrimitive(autoRequested),
    )

    /** 最後の状態の置き場に書く文字列。キーの順が毎回同じなので、文字列の比べで同じ状態かが分かる。 */
    internal fun canonical(): String = ingestJson.encodeToString(JsonObject(fields()))

    companion object {
        fun of(reading: HealthReading, autoRequested: Boolean): HealthState {
            val usable = reading.availability == HealthAvailability.AVAILABLE
            return HealthState(
                availability = reading.availability,
                features = if (usable) reading.features else HealthFeatures(history = false, background = false),
                granted = HEALTH_PERMISSIONS.mapValuesTo(linkedMapOf()) { (_, name) ->
                    usable && name in reading.grantedPermissions
                },
                autoRequested = autoRequested,
            )
        }
    }
}

/**
 * 観た状態から `c01-health-permission` の記録を 1 件作る（design D2）。`external_id` なし・`schema_version = 1`。
 * 出来事の時刻は観た時の端末の壁時計（補正しない）。原文は `payload` と同じ JSON を直列化した文字列。
 * 欄とキーの順は冪等キーの入力なので変えない（変えるなら `schema_version` を上げる）。
 * [errorType] は `unreadable` のときの例外の型名だけ。
 */
fun healthPermissionRecord(
    state: HealthState,
    trigger: String,
    errorType: String?,
    clock: DeviceClock,
    deviceId: String,
    userId: String,
    zone: ZoneId,
    newId: () -> String,
): IngestRequest {
    val deviceMs = clock.wallMs()
    val fields = linkedMapOf<String, kotlinx.serialization.json.JsonElement>(
        "kind" to JsonPrimitive("health-permission"),
        "trigger" to JsonPrimitive(trigger),
        "device_time" to JsonPrimitive(clockInstantText(deviceMs)),
        "elapsed_ms" to JsonPrimitive(clock.monoMs()),
        "boot_count" to (clock.bootCount()?.let { JsonPrimitive(it) } ?: JsonNull),
    )
    fields.putAll(state.fields())
    errorType?.let { fields["error"] = JsonPrimitive(it) }
    return IngestRequest(
        id = newId(),
        userId = userId,
        logicalSource = HEALTH_LOGICAL_SOURCE,
        externalId = null,
        deviceId = deviceId,
        origin = "collected",
        eventTime = clockInstantText(deviceMs),
        tzOffsetMin = zone.rules.getOffset(Instant.ofEpochMilli(deviceMs)).totalSeconds / 60,
        tzId = zone.id,
        schemaVersion = 1,
        unitSystem = null,
        crs = null,
        raw = ingestJson.encodeToString(JsonObject(fields)),
        payload = JsonObject(fields),
    )
}

/**
 * 最後に記録した状態（判定に使う 4 欄だけ）の置き場（design D2 / D14）。[HEALTH_PREFS] に持ち、自動バックアップから外す。
 * 時刻は持たない —— 履歴は S-01 が持つ。
 */
class HealthStateStore(private val prefs: SharedPreferences) {
    fun last(): String? = prefs.getString(KEY_LAST_STATE, null)

    /** 書けたか。書けなくても落とさない（次に観たときにもう一度作る側に倒れる）。 */
    fun remember(state: HealthState): Boolean = prefs.edit().putString(KEY_LAST_STATE, state.canonical()).commit()

    /**
     * 最後の状態を忘れる（review/code.md R19 / design D15（仮））。未送信から `c01-health-permission` の記録が捨てられたとき
     * （保持の上限）に呼ぶ —— 捨てた記録の状態は S-01 に届かないので、次に観たとき必ず 1 件作り直させる。
     */
    fun forget(): Boolean = prefs.edit().remove(KEY_LAST_STATE).commit()

    private companion object {
        const val KEY_LAST_STATE = "last_state"
    }
}

/**
 * 「観る → 前と違えば 1 件を未送信に積み、積めたら最後の状態を書き換える」（design D2）。
 *
 * - 判定は [HealthState] の 4 欄だけ。契機・時計・経過・起動の識別は使わない（時計が戻っても進んでも、変わっていなければ作らない）
 * - 最後の状態が無ければ（入れ直した後を含む）必ず 1 件
 * - [emit] は未送信に積めたかを返す。**積めなかったら最後の状態を書き換えない**
 * - 画面とサービスが別々の recorder で**同じ置き場を共有する**（review/code.md R1）。比べる相手は置き場で、
 *   recorder ごとの控えは「積んだが置き場への書き換えを遅らせている間」だけ持つ（書き換えが済んだら捨てる）。
 *   控えを持ち続けると、他方が置き場を書き換えた後に変化を取りこぼすか、同じ状態を重ねて積む
 * - 画面の控えは Activity が作り直されても残るよう、プロセスで 1 つの [HealthPendingSlot] に置く（review/code.md R24）
 * - ログは `health_permission_recorded` の契機と使える状態だけ。種類ごとの許可の値は出さない
 * - 生存信号は送らず、[CollectionSource] にも入れない（design D11）
 */
class HealthPermissionRecorder(
    private val read: suspend () -> HealthReading,
    private val autoRequested: () -> Boolean,
    private val store: HealthStateStore,
    private val clock: DeviceClock,
    private val deviceId: String,
    private val userId: String,
    private val zone: ZoneId,
    private val newId: () -> String,
    private val emit: (IngestRequest) -> Boolean,
    private val log: (String) -> Unit = {},
    /**
     * 最後の状態を書き換え、**書き換えを試みた後に** `settled` を呼ぶ（書けたかに依らない —— 書けなければ置き場が古いまま残り、
     * 次に観たときにもう一度作る側に倒れる）。既定は即時。画面は未送信に書けた後まで遅らせる（[HealthRecordHandoff.afterDrain]）。
     */
    private val remember: (state: HealthState, settled: () -> Unit) -> Unit = { state, settled ->
        store.remember(state)
        settled()
    },
    /** 積んだが置き場への書き換えがまだ済んでいない状態の控え。既定は recorder ごと。錠も兼ねる。 */
    private val pending: HealthPendingSlot = HealthPendingSlot(),
) {
    /** 観て、前と違えば積む。**読んだ状態を返す**（画面が求めるかの判断に同じ読みを使う。review/code.md R28）。 */
    suspend fun observeHealthPermissions(trigger: String): HealthReading {
        val reading = read()
        val state = HealthState.of(reading, autoRequested())
        // 読み口は主スレッドを塞がないよう錠の外。比べて積んで書き換えるところは、2 つの契機が重なっても 1 件にする
        synchronized(pending) { recordIfChanged(state, trigger, reading.errorType) }
        return reading
    }

    /** 置き場（か控え）と違えば 1 件積み、積めたら書き換える。[pending] の錠の中で呼ぶ。 */
    private fun recordIfChanged(state: HealthState, trigger: String, errorType: String?) {
        val canonical = state.canonical()
        if ((pending.value ?: store.last()) == canonical) return
        val record = healthPermissionRecord(state, trigger, errorType, clock, deviceId, userId, zone, newId)
        if (!emit(record)) return
        pending.value = canonical
        remember(state) {
            // 遅らせた書き換えは別の糸（サービスの起動）から呼ばれる。後から積んだ別の状態の控えは消さない
            synchronized(pending) { if (pending.value == canonical) pending.value = null }
        }
        log(
            Telemetry.line(
                "health_permission_recorded",
                source = HEALTH_LOGICAL_SOURCE,
                trigger = trigger,
                availability = state.availability.wire,
            ),
        )
    }
}

/**
 * 積んだが置き場への書き換えがまだ済んでいない状態（遅らせている間だけ。済んだら null に戻す）。
 * 預けてから書き換えるまでに同じ状態をもう一度観ても重ねて積まないための控え。読み書きはこのオブジェクトの錠の中で行う。
 */
class HealthPendingSlot {
    var value: String? = null
}
