// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.location.LocationManager
import android.net.ConnectivityManager
import android.net.NetworkCapabilities

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
    permission = hasLocationPermission(context),
    sensor = hasLocationProvider(context),
    network = hasNetwork(context),
)

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
