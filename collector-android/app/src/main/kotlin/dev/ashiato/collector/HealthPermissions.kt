// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

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
