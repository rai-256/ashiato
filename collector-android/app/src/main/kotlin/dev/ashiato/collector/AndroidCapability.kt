// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.app.AppOpsManager
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.location.LocationManager
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import android.os.Process
import android.provider.Settings

/**
 * いま位置を取得できる状態かを端末から読む（深掘り Q5 / tasks 7.2）。
 *
 * **稼働だけを送っていては足りない。** Android は長期間使っていないアプリの権限を
 * 自動で剥がす。位置の権限が剥がれると**収集プロセスは生きたまま位置が 0 件**になり、
 * 「動いている」だけを送っていると壊れているのに「動いていた」と記録が残る。
 * 一次情報が裏付けている —— `docs/requirements.md` の EXT-D
 * 「写真の EXIF 位置が読めるかは、撮影時期ではなく**実行時点の権限状態**で決まる」。
 *
 * **読むだけで、直そうとしない。** 権限の要求は `MainActivity` の仕事で、
 * ここは「いまどうなっているか」を報告する責務だけを持つ。
 */
fun androidCapability(context: Context): Capability = Capability.of(
    permission = runCatching { hasLocationPermission(context) }.getOrDefault(false),
    sensor = runCatching { hasLocationProvider(context) }.getOrDefault(false),
    network = runCatching { hasNetwork(context) }.getOrDefault(false),
)

// **読めなかったら「取れない」に倒す**（review/code.md の R31）。
// 端末の状態を読む口はどれも SecurityException を投げうる（宣言漏れ・端末の方言）。
// 投げると起動時の生存信号がそれを貫通し、START_STICKY と合わさって**クラッシュループ**になる。
// 倒す向きは「取れない」側 —— 読めていないのに「取れている」と報告するのは、
// 壊れているのに「動いていた」と残すのと同じ（深掘り Q5 が塞いだ当の型）。
// 理由は `Capability.of` が必ず埋めるので、「理由の無い取れない」にはならない。

/**
 * 位置の権限。**粗い位置だけでは足りない** —— FR-1 は 60 秒ごとの位置を求めており、
 * `ACCESS_COARSE_LOCATION` だけに落とされた状態は「取れている」とは言えない。
 */
private fun hasLocationPermission(context: Context): Boolean =
    context.checkSelfPermission(Manifest.permission.ACCESS_FINE_LOCATION) ==
        PackageManager.PERMISSION_GRANTED

/**
 * 位置の取得元が有効か。**本人が端末の位置情報そのものを切っている**ときは、
 * 権限があっても 1 件も取れない。
 */
private fun hasLocationProvider(context: Context): Boolean {
    val manager = context.getSystemService(LocationManager::class.java) ?: return false
    return manager.isLocationEnabled
}

/**
 * S-01 へ届く網があるか。
 *
 * 位置の取得そのものは網に依存しないが、**送れない期間の記録は端末内にしか無い** ——
 * ST04 の保持上限を超えれば破棄される。取得の健全性と同じ列に載せる理由がここにある。
 */
private fun hasNetwork(context: Context): Boolean {
    val manager = context.getSystemService(ConnectivityManager::class.java) ?: return false
    val caps = manager.getNetworkCapabilities(manager.activeNetwork) ?: return false
    return caps.hasCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)
}

/**
 * いまアプリ利用（`UsageStatsManager`）を取得できる状態かを端末から読む（tasks 5.2 / 本人の決定 C9）。
 *
 * **「利用状況へのアクセス」は実行時権限ではない。** `PACKAGE_USAGE_STATS` は宣言しても付かず
 * （付けられるのは設定アプリの「特別なアクセス」だけ）、`checkSelfPermission` は端末によって
 * **許可済みでも拒否を返し続ける**。本人が許した事実を読めるのは `AppOpsManager` の
 * `GET_USAGE_STATS` の側だけで、ここを取り違えると許可済みの端末が「取れない」を送り続ける。
 *
 * **`sensor` は持たない**（本人の決定 C9）—— アプリ利用に当たるものが無いので、
 * `blockers` は `permission` と `network` の 2 つだけになる。位置の口（[androidCapability]）を
 * 使い回すと、端末の位置情報が切られているだけでアプリ利用が「センサが無い」と報告する。
 *
 * **読めなかったら「取れない」に倒す**（[androidCapability] と同じ規律。review/code.md の R31）。
 */
fun usageAccessCapability(context: Context): Capability = Capability.of(
    permission = runCatching { hasUsageAccess(context) }.getOrDefault(false),
    // 満たされない道が無いので常に真。**`Capability.of` の口はそのまま使う**
    // （`blockers` の作り方を 2 通りにすると、名前の綴りが 2 か所に散る）
    sensor = true,
    network = runCatching { hasNetwork(context) }.getOrDefault(false),
)

/**
 * 「利用状況へのアクセス」が本人に許されているか。
 *
 * `MODE_DEFAULT` は**本人がまだ 1 度も触っていない**という意味で、「許された」ではない ——
 * そのときだけ宣言した権限の付与状態（`adb` や端末の管理者が直に付ける道がある）を見る。
 * 既定を「許された」に倒すと、初回起動の端末が全部「取れている」と名乗り、
 * 1 件も取れていない期間が③（取れない状態）ではなく①として残る。
 */
private fun hasUsageAccess(context: Context): Boolean {
    val ops = context.getSystemService(AppOpsManager::class.java) ?: return false
    val mode = ops.unsafeCheckOpNoThrow(
        AppOpsManager.OPSTR_GET_USAGE_STATS,
        Process.myUid(),
        context.packageName,
    )
    if (mode != AppOpsManager.MODE_DEFAULT) return mode == AppOpsManager.MODE_ALLOWED
    return context.checkSelfPermission(Manifest.permission.PACKAGE_USAGE_STATS) ==
        PackageManager.PERMISSION_GRANTED
}

/**
 * 「利用状況へのアクセス」の設定画面へ行く指示（tasks 5.3 / 本人の決定 Q3 / design D6（仮））。
 *
 * **この特別なアクセスは、位置のようなその場のダイアログでは取れない** ——
 * アプリから開けるのはこの設定画面までで、許すかどうかは本人が端末の設定で決める。
 *
 * 初回起動の [MainActivity] と、常駐の通知（[LocationService]）の**両方から同じ画面へ送る**ので、
 * 指示の組み立てはここ 1 か所に置く（2 か所に書くと、片方だけ別の画面へ行く）。
 */
fun usageAccessSettingsIntent(): Intent = Intent(Settings.ACTION_USAGE_ACCESS_SETTINGS)
    // 常駐の通知から開く道（`PendingIntent`）はアプリの画面の外から始まるので、自分の入れ物が要る
    .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
