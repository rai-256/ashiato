// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.content.Context
import android.content.pm.PackageManager

/**
 * パッケージ名から**取得時点の**アプリの表示名を引く（本人の決定 C2 / design D2）。
 *
 * **口にしてあるのは、端末に触らずに記録の組み立てを試験するため**（`FixSource` と同じ形）。
 * 引けたものは `payload` にだけ入る —— `raw` に入れると、アプリの更新や端末の言語の変更で
 * 冪等キーが動き、**同じイベントが行を増やす**。
 */
fun interface AppLabels {
    /** 引けなければ null（アプリが消えている・パッケージ名が無い）。**推測で埋めない。** */
    fun label(packageName: String?): String?
}

/**
 * 端末の `PackageManager` から引く本番の実装。
 *
 * **同じ契機の中で同じアプリが何度も出る**（前景の出入りは対で来る）ので、
 * 引けた結果も引けなかった結果も覚える —— `getApplicationInfo` は端末への問い合わせで、
 * 1 契機に数百件を素で引くと取得そのものが遅くなる。
 *
 * 覚えているのはプロセスが生きている間だけ。アプリを更新すると表示名が変わりうるが、
 * **`payload` は取得時点の値でよい**（凍結の対象は原文のほうで、表示名は後から引き直せる）。
 */
class PackageManagerAppLabels(context: Context) : AppLabels {
    private val packages: PackageManager = context.packageManager
    private val seen = HashMap<String, String?>()

    override fun label(packageName: String?): String? {
        val name = packageName ?: return null
        // **引けなかったことも覚える**（`getOrPut` では null を覚えられず、消えたアプリを毎回引き直す）
        if (seen.containsKey(name)) return seen[name]
        val label = try {
            packages.getApplicationInfo(name, 0).loadLabel(packages).toString()
        } catch (e: PackageManager.NameNotFoundException) {
            // 消えたアプリ。**空文字で埋めない** —— 引けなかったことは欄を省くことで残す
            null
        }
        seen[name] = label
        return label
    }
}
