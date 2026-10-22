// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.content.Context
import android.content.SharedPreferences
import androidx.core.content.IntentCompat
import androidx.core.content.PackageManagerCompat
import androidx.core.content.UnusedAppRestrictionsConstants
import java.util.concurrent.TimeUnit

/**
 * 「使っていないアプリの一時停止」の対象から外れているか（design D12（仮）/ Q4）。
 *
 * OS は数か月開かれないアプリの権限を剥がし、通知を止める。収集アプリは前景サービスで動くだけで
 * 「使った」に数えられないので、外れていなければ止まりうる。**読んで、外れていなければ頼む。**
 * [wire] は生存信号の原文の `unused_app_restrictions` の値（契約 `docs/collector-contract.md`）。
 */
enum class UnusedAppRestrictionsState(val wire: String) {
    /** 対象から外れている。頼まない・何も出さない。 */
    EXEMPT("exempt"),

    /** 対象から外れていない。頼む。 */
    NOT_EXEMPT("not_exempt"),

    /** 仕組みを持たない端末。頼まない・何も出さない。 */
    UNSUPPORTED("unsupported"),

    /** 読めなかった。頼まず、読めなかったことを画面に出す。 */
    UNKNOWN("unknown"),
}

/** `PackageManagerCompat` の結果を 4 つの状態に写す。**知らない値は読めなかったことにする。** */
fun mapUnusedAppRestrictionsStatus(status: Int): UnusedAppRestrictionsState = when (status) {
    UnusedAppRestrictionsConstants.DISABLED -> UnusedAppRestrictionsState.EXEMPT
    UnusedAppRestrictionsConstants.API_30_BACKPORT,
    UnusedAppRestrictionsConstants.API_30,
    UnusedAppRestrictionsConstants.API_31,
    -> UnusedAppRestrictionsState.NOT_EXEMPT
    UnusedAppRestrictionsConstants.FEATURE_NOT_AVAILABLE -> UnusedAppRestrictionsState.UNSUPPORTED
    else -> UnusedAppRestrictionsState.UNKNOWN
}

/** 状態の読み口。**試験が差し替える**（Robolectric は OS の除外の状態を持たない）。 */
fun interface UnusedAppRestrictionsReader {
    fun read(context: Context): UnusedAppRestrictionsState
}

/**
 * 本物の読み口。minSdk 30 では結果がすぐ揃う（`API_30` 以降の経路は同期で完了する）ので、
 * 短い待ちで足りる。**待ちきれない・例外・`ERROR` は読めなかったことにする**（落とさない）。
 */
object SystemUnusedAppRestrictionsReader : UnusedAppRestrictionsReader {
    private const val WAIT_MS = 2_000L

    // 何が投げられても落とさず「読めなかった」に写すのが仕様（画面に読めなかったことを出す）
    @Suppress("TooGenericExceptionCaught", "SwallowedException")
    override fun read(context: Context): UnusedAppRestrictionsState = try {
        val status = PackageManagerCompat.getUnusedAppRestrictionsStatus(context)
            .get(WAIT_MS, TimeUnit.MILLISECONDS)
        mapUnusedAppRestrictionsStatus(status)
    } catch (e: Exception) {
        UnusedAppRestrictionsState.UNKNOWN
    }
}

/** 設定の画面（収集アプリを対象から外す）を開く `Intent`。 */
fun manageUnusedAppRestrictionsIntent(context: Context) =
    IntentCompat.createManageUnusedAppRestrictionsIntent(context, context.packageName)

/**
 * 自動で頼んだ印（端末に残す）。**1 つの印だけ**で、新しく入れた端末も前の版から更新した端末も
 * 印を持たないので、最初の起動で 1 回だけ頼む（spec-review R14）。
 */
class UnusedAppRestrictionsAsked(private val prefs: SharedPreferences) {
    fun asked(): Boolean = prefs.getBoolean(KEY, false)

    fun mark() = prefs.edit().putBoolean(KEY, true).apply()

    companion object {
        const val KEY = "asked_unused_app_restrictions"
    }
}
