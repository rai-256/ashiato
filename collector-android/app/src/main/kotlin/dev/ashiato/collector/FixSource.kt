// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.content.Context
import android.os.Looper
import com.google.android.gms.location.LocationCallback
import com.google.android.gms.location.LocationRequest
import com.google.android.gms.location.LocationServices
import com.google.android.gms.location.Priority

/**
 * 位置の契機を配る口。
 *
 * **`LocationService` から切り出してあるのは試験のため**（review R1）。
 * 切り出す前は、取得の間隔を 60 秒から 1 秒に変えても・送信を 5 分から 10 秒に変えても・
 * 永続 Outbox をメモリだけの置き場に戻しても、**単体 52 件が全部緑のまま通った**。
 * 本人が決めた値（design D7 / D9）と、深掘り 第 2 回で戻した欠陥の修正が、
 * 部品の層でしか固定されていなかった。
 */
interface FixSource {
    /** 契機を配り始める。権限が無ければ [SecurityException]。 */
    fun start(intervalMs: Long, callback: LocationCallback)
    fun stop(callback: LocationCallback)
}

/** 本番の取得元。Play Services（design D7 / D15）。 */
class FusedFixSource(context: Context) : FixSource {
    private val client = LocationServices.getFusedLocationProviderClient(context)

    override fun start(intervalMs: Long, callback: LocationCallback) {
        val request = LocationRequest.Builder(Priority.PRIORITY_HIGH_ACCURACY, intervalMs).build()
        client.requestLocationUpdates(request, callback, Looper.getMainLooper())
    }

    override fun stop(callback: LocationCallback) {
        client.removeLocationUpdates(callback)
    }
}

/** 送信の契機を刻む口。**周期を試験から観測できるようにするため**に切り出してある（review R1）。 */
interface FlushScheduler {
    fun every(periodMs: Long, task: () -> Unit)
    fun cancel()
}
