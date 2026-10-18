// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build

/**
 * メディアの `blockers` の語彙（design D6）。C-01 の `permission` / `sensor` / `network` と混ぜない別の語で、
 * 位置・アプリ利用の語（[Capability.PERMISSION] など）の綴りを使うのは `permission` だけ（同じ意味: 権限が無い）。
 * サーバは語彙を検査しない。
 */
object MediaBlocker {
    /** 写真と動画へのアクセスが無い */
    const val PERMISSION = Capability.PERMISSION

    /** `ACCESS_MEDIA_LOCATION` が無い（Q15） */
    const val MEDIA_LOCATION = "media_location"

    /** 「選んだ写真だけ」（C12） */
    const val PARTIAL = "partial"
}

/**
 * 取得から送信待ちまでの段（C19 / design D6・D22）。読めたのに失敗し続ける段を、その段の語で報告する。
 * 原本の印の積み込み（Task 5）は、原本の送り待ちの台帳を持つ [LEDGER] の側が引き受ける。
 */
enum class MediaStage(val word: String) {
    /** ファイルを開けない・メタを読めない */
    READ("stage_read"),

    /** 内容ハッシュを計算できない */
    HASH("stage_hash"),

    /** 記録を未送信に積めない */
    ENQUEUE("stage_enqueue"),

    /** 見分けの台帳・読み進めた印を書けない */
    LEDGER("stage_ledger"),
}

/**
 * 1 本のメディアのソースの、生存信号の 1 区間ぶんの失敗の段（D22）。
 *
 * 「その区間のすべての試行が同じ段で失敗した」ときだけ段の語を載せる —— 成功が 1 回でもあれば載せない。
 * 失敗の段が途中で変わったら最後の段（C19）。**区間の終わり（生存信号を積めたとき）に [reset] する。**
 * 端末の保存領域には置かない（プロセスの立て直しで区間の記憶が消えても、載るはずの段の語が 1 区間ぶん欠けるだけ）。
 */
class MediaStageLog {
    private var successes = 0
    private var failures = 0
    private var lastStage: MediaStage? = null

    @Synchronized
    fun succeeded() {
        successes++
    }

    /** 失敗。[stage] が null なのは段に入る前（権限が無くて何も読んでいない）。 */
    @Synchronized
    fun failed(stage: MediaStage?) {
        failures++
        if (stage != null) lastStage = stage
    }

    /** この区間のすべての試行が失敗し、段が分かっているときのその段 */
    @Synchronized
    fun blockedStage(): MediaStage? = if (successes == 0 && failures > 0) lastStage else null

    @Synchronized
    fun reset() {
        successes = 0
        failures = 0
        lastStage = null
    }
}

/** 写真と動画へのアクセスと写真の位置の権限（取得可否と知らせの元。[MediaAccess.NO_LOCATION] はここでは使わない）。 */
data class MediaPermissionState(val access: MediaAccess, val location: Boolean) {
    init {
        require(access != MediaAccess.NO_LOCATION) { "位置の権限は location で持つ" }
    }

    /** 読み手へ渡す状態。位置の権限が無ければ、それ以外の状態より先に [MediaAccess.NO_LOCATION]（Q15） */
    fun forScanner(): MediaAccess = if (!location) MediaAccess.NO_LOCATION else access
}

/**
 * メディアの 4 つのソースの取得可否（design D6 / D7）。`blockers` は D6 の語だけ。**接続は含めない**（`network` は別の欄）。
 * [stage] は [MediaStageLog.blockedStage]。
 */
fun mediaCapability(state: MediaPermissionState, stage: MediaStage? = null): Capability {
    val blockers = buildList {
        if (state.access == MediaAccess.NONE) add(MediaBlocker.PERMISSION)
        if (!state.location) add(MediaBlocker.MEDIA_LOCATION)
        if (state.access == MediaAccess.PARTIAL) add(MediaBlocker.PARTIAL)
        if (stage != null) add(stage.word)
    }
    return Capability(blockers.isEmpty(), blockers)
}

/** 初回の起動で求める、写真と動画へのアクセスの権限（design D6）。33 以上は画像と動画、14 以降はさらに「選んだ写真だけ」。 */
fun mediaAccessPermissions(sdk: Int): List<String> = when {
    sdk >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE -> listOf(
        Manifest.permission.READ_MEDIA_IMAGES,
        Manifest.permission.READ_MEDIA_VIDEO,
        Manifest.permission.READ_MEDIA_VISUAL_USER_SELECTED,
    )
    sdk >= Build.VERSION_CODES.TIRAMISU -> listOf(
        Manifest.permission.READ_MEDIA_IMAGES,
        Manifest.permission.READ_MEDIA_VIDEO,
    )
    else -> listOf(Manifest.permission.READ_EXTERNAL_STORAGE)
}

/** 位置の後に 1 度にまとめて求めるもの: アクセスの権限と、写真の位置（`ACCESS_MEDIA_LOCATION`） */
fun mediaPermissionsToAsk(sdk: Int): List<String> =
    mediaAccessPermissions(sdk) + Manifest.permission.ACCESS_MEDIA_LOCATION

/**
 * いまの権限を端末から読む。**読むだけで、直そうとしない**（求めるのは入口の仕事）。
 * 読めなければ「何も無い」に倒す（読めていないのに取れていると報告しない。[androidCapability] と同じ規律）。
 */
fun mediaPermissionState(context: Context, sdk: Int = Build.VERSION.SDK_INT): MediaPermissionState {
    fun has(permission: String) = runCatching {
        context.checkSelfPermission(permission) == PackageManager.PERMISSION_GRANTED
    }.getOrDefault(false)
    val access = when {
        sdk >= Build.VERSION_CODES.TIRAMISU -> {
            val images = has(Manifest.permission.READ_MEDIA_IMAGES)
            val video = has(Manifest.permission.READ_MEDIA_VIDEO)
            val selected = sdk >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE &&
                has(Manifest.permission.READ_MEDIA_VISUAL_USER_SELECTED)
            when {
                images && video -> MediaAccess.FULL
                // 片方だけ・「選んだ写真だけ」（14 はこの 1 本だけが付く）。一覧に見えない種類があるので消えたかの判定はしない
                images || video || selected -> MediaAccess.PARTIAL
                else -> MediaAccess.NONE
            }
        }
        has(Manifest.permission.READ_EXTERNAL_STORAGE) -> MediaAccess.FULL
        else -> MediaAccess.NONE
    }
    return MediaPermissionState(access, has(Manifest.permission.ACCESS_MEDIA_LOCATION))
}
