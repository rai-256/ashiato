// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.content.Context
import android.content.Intent
import android.os.Build
import androidx.health.connect.client.HealthConnectClient
import androidx.health.connect.client.HealthConnectFeatures
import androidx.health.connect.client.PermissionController
import kotlinx.coroutines.CancellationException

/**
 * 健康データで求める権限（design D5（仮））。鍵は記録の `granted` の欄の名前（design D2）、値は権限の名前。
 * manifest が 6 つともちょうど宣言していることを `HealthManifestTest` が見る（宣言していない権限は求めても画面に出ない）。
 * 順序は記録のキーの順なので変えない。
 */
val HEALTH_PERMISSIONS: Map<String, String> = linkedMapOf(
    "sleep" to "android.permission.health.READ_SLEEP",
    "heart_rate" to "android.permission.health.READ_HEART_RATE",
    "steps" to "android.permission.health.READ_STEPS",
    "exercise" to "android.permission.health.READ_EXERCISE",
    "history" to "android.permission.health.READ_HEALTH_DATA_HISTORY",
    "background" to "android.permission.health.READ_HEALTH_DATA_IN_BACKGROUND",
)

/**
 * 健康データの印と最後の状態を置く `SharedPreferences` の名前（design D14（仮））。
 * 自動バックアップと端末の移行から外すので、`collector` の置き場には置かない。
 * `res/xml/` の 2 つの規則の `path` と一致していることを `HealthManifestTest` が見る。
 */
const val HEALTH_PREFS = "health_permission"

/**
 * 健康データの権限を自動で求めて、結果が返ったことの印の鍵（design D4（仮））。[HEALTH_PREFS] に置く。
 * 結果が返ったときに立てる（開いたときに立てると、答えずにプロセスが終わったとき二度と出ない）。
 * 前の版には無い鍵なので、更新の後の最初の起動で 1 度求める。
 */
const val ASKED_HEALTH_PERMISSIONS = "asked_health_permissions"

/** 常駐の通知の操作のラベル（design D8（仮））。 */
const val HEALTH_PERMISSION_ACTION_LABEL = "健康データの権限"

/**
 * 常駐の通知の操作が開く画面（design D8（仮）/ 深掘り Q11）。14 以降は Health Connect の設定のトップ
 * （`HealthConnectClient.ACTION_HEALTH_CONNECT_SETTINGS`）で、本人がそこから「アプリの権限 → あしあと。」へたどる。
 * 収集アプリの権限の画面（`ACTION_MANAGE_HEALTH_PERMISSIONS`）は OS の特別な権限が要り、普通のアプリからは開けない
 * （計測テストの実測。review/code.md R12）。13 以前は Health Connect のアプリの画面（client の口）。
 */
fun healthPermissionSettingsIntent(context: Context, sdkInt: Int = Build.VERSION.SDK_INT): Intent =
    if (sdkInt >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
        Intent(HealthConnectClient.ACTION_HEALTH_CONNECT_SETTINGS)
    } else {
        HealthConnectClient.getHealthConnectManageDataIntent(context)
    }

/** Health Connect が使えるか（design D6）。[wire] は記録の `availability` の値（design D2）。 */
enum class HealthAvailability(val wire: String) {
    AVAILABLE("available"),
    NOT_INSTALLED("not_installed"),
    UPDATE_REQUIRED("update_required"),
    UNREADABLE("unreadable"),
}

/** 履歴と背景の読み取りに、この端末の Health Connect が対応しているか（design D3）。 */
data class HealthFeatures(val history: Boolean, val background: Boolean)

/**
 * 読み口が返す、いまの実際の状態。[grantedPermissions] は許されている権限の名前
 * （[HEALTH_PERMISSIONS] の値のうちのもの）。[errorType] は `UNREADABLE` のときの例外の型名だけ
 * （メッセージは持たない。ログに許されているものの中身を出さない）。
 */
data class HealthReading(
    val availability: HealthAvailability,
    val features: HealthFeatures,
    val grantedPermissions: Set<String>,
    val errorType: String? = null,
)

/**
 * 健康データの権限の読み口と求める口（design D3（仮））。**試験が差し替える**（Robolectric は Health Connect を持たない）。
 * `MainActivity` の `healthPermissions()` で返す。健康データそのものは読まない。
 */
interface HealthPermissions {
    /** いまの使える状態・対応・許されている権限。**例外は投げず `UNREADABLE` に倒す。** */
    suspend fun read(context: Context): HealthReading

    /** 権限を求める画面を開く intent。 */
    fun createRequestIntent(context: Context, permissions: Set<String>): Intent

    /** 求めた結果から許された権限を取り出す。使い道は限られる —— 実際の状態は [read] で読み直す。 */
    fun parseResult(resultCode: Int, data: Intent?): Set<String>
}

/**
 * 求めるもの（design D6 の表）。使えるときだけ、4 種類 + 対応している履歴・背景。
 * 無い・更新が要る・読めないときは求めない。順序は [HEALTH_PERMISSIONS] に従う。
 */
fun healthPermissionsToAsk(availability: HealthAvailability, features: HealthFeatures): List<String> {
    if (availability != HealthAvailability.AVAILABLE) return emptyList()
    return HEALTH_PERMISSIONS.filterKeys { key ->
        when (key) {
            "history" -> features.history
            "background" -> features.background
            else -> true
        }
    }.values.toList()
}

/** `HealthConnectClient` のうち使う 3 つの口（試験が偽物を渡せるようにする）。 */
interface HealthClientPort {
    /** `HealthConnectClient.SDK_*` の値。 */
    fun sdkStatus(context: Context): Int
    suspend fun featureAvailable(context: Context, feature: Int): Boolean
    suspend fun grantedPermissions(context: Context): Set<String>
}

private object RealHealthClientPort : HealthClientPort {
    private fun client(context: Context) = HealthConnectClient.getOrCreate(context.applicationContext)

    override fun sdkStatus(context: Context): Int = HealthConnectClient.getSdkStatus(context)

    override suspend fun featureAvailable(context: Context, feature: Int): Boolean =
        client(context).features.getFeatureStatus(feature) == HealthConnectFeatures.FEATURE_STATUS_AVAILABLE

    override suspend fun grantedPermissions(context: Context): Set<String> =
        client(context).permissionController.getGrantedPermissions()
}

/** client を使う本番の実装。 */
class ClientHealthPermissions(private val port: HealthClientPort = RealHealthClientPort) : HealthPermissions {
    // 何が投げられても落とさず `unreadable`（例外の型名だけ）に写すのが仕様（design D6）
    @Suppress("TooGenericExceptionCaught")
    override suspend fun read(context: Context): HealthReading = try {
        when (port.sdkStatus(context)) {
            HealthConnectClient.SDK_AVAILABLE -> readAvailable(context)
            HealthConnectClient.SDK_UNAVAILABLE_PROVIDER_UPDATE_REQUIRED ->
                HealthReading(HealthAvailability.UPDATE_REQUIRED, NO_FEATURES, emptySet())
            else -> HealthReading(HealthAvailability.NOT_INSTALLED, NO_FEATURES, emptySet())
        }
    } catch (e: CancellationException) {
        throw e
    } catch (e: Exception) {
        HealthReading(HealthAvailability.UNREADABLE, NO_FEATURES, emptySet(), e.javaClass.name)
    }

    private suspend fun readAvailable(context: Context): HealthReading {
        val features = HealthFeatures(
            history = port.featureAvailable(context, HealthConnectFeatures.FEATURE_READ_HEALTH_DATA_HISTORY),
            background = port.featureAvailable(context, HealthConnectFeatures.FEATURE_READ_HEALTH_DATA_IN_BACKGROUND),
        )
        val granted = port.grantedPermissions(context).filter { it in HEALTH_PERMISSIONS.values }.toSet()
        return HealthReading(HealthAvailability.AVAILABLE, features, granted)
    }

    override fun createRequestIntent(context: Context, permissions: Set<String>): Intent =
        PermissionController.createRequestPermissionResultContract().createIntent(context, permissions)

    override fun parseResult(resultCode: Int, data: Intent?): Set<String> =
        PermissionController.createRequestPermissionResultContract().parseResult(resultCode, data)

    private companion object {
        val NO_FEATURES = HealthFeatures(history = false, background = false)
    }
}
