// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.app.NotificationCompat
import java.time.Duration

/**
 * 「S-01 の更新を待っている」ときの案内（ST50 / design D6・D22・D24（仮））。画面の行・通知・上限の 7 日前の知らせが同じ文言を使う。
 * 理由が 200 の読めない答え（e）だけのときは、公衆の Wi-Fi の入口でも誤りにならない文言にする。
 */
data class ContractNotice(
    val onlyUnreadable: Boolean,
    val days: Int,
    val channelMismatch: Boolean = false,
    /** 系統不一致だけの行では、ST50 の契約更新文を混ぜない。 */
    val contractWaiting: Boolean = true,
    /** 自分の系統（系統の違いの文の相手を決める）。試験は差し替える */
    val local: Channel = buildLocalChannel(),
) {
    val headline: String get() = if (channelMismatch) {
        if (contractWaiting) "${contractHeadline(onlyUnreadable)}。${channelMismatchHeadline(local)}"
        else channelMismatchHeadline(local)
    } else contractHeadline(onlyUnreadable)

    /** 画面の 1 行と通知の本文。系統の違いだけの行は日数を添えない（契約の版の待ちの日数ではないので。final review R7） */
    val line: String get() = if (channelMismatch && !contractWaiting) headline
        else "$headline（未送信 $days 日分を携帯に残しています）"
}

const val CONTRACT_UPDATE_HEADLINE = "サーバを更新してください"
const val CONTRACT_NETWORK_HEADLINE = "サーバを更新するか、つながっている網を確かめてください"

fun channelMismatchHeadline(local: Channel): String =
    "送り先が${if (local == Channel.Dev) "正式版" else "開発版"}のサーバです。送り先を確かめてください"

fun contractHeadline(onlyUnreadable: Boolean): String =
    if (onlyUnreadable) CONTRACT_NETWORK_HEADLINE else CONTRACT_UPDATE_HEADLINE

/** 手元に残している最も古い記録を積んでからの日数（切り捨て。無ければ 0。D6） */
fun oldestUnsentDays(records: Outbox<*>, age: () -> Long): Int =
    records.oldest()?.let { ((age() - it.enqAgeMs) / AgeClock.DAY_MS).toInt() }?.coerceAtLeast(0) ?: 0

/** 止まった状態の写しを作る。止まっていなければ null。 */
fun contractNotice(reasons: ContractReasons, days: Int): ContractNotice? =
    if (reasons.waiting) ContractNotice(reasons.onlyUnreadable, days) else null

class ChannelMismatchNotifier(
    private val context: Context,
    private val store: ContractWaitStore,
    private val now: () -> Long = System::currentTimeMillis,
    /** 自分の系統。試験は差し替える */
    private val local: Channel = buildLocalChannel(),
) {
    private val manager: NotificationManager get() = context.getSystemService(NotificationManager::class.java)

    fun update(mismatch: Boolean) {
        if (!mismatch) {
            if (store.channelNotifiedAtMs != null) {
                manager.cancel(CHANNEL_NOTIFICATION_ID)
                store.channelNotifiedAtMs = null
            }
            return
        }
        val last = store.channelNotifiedAtMs
        if (last != null && now() - last < ContractNotifier.REPEAT.toMillis()) return
        manager.createNotificationChannel(NotificationChannel(ContractNotifier.CHANNEL, "サーバの更新", NotificationManager.IMPORTANCE_DEFAULT))
        manager.notify(CHANNEL_NOTIFICATION_ID, NotificationCompat.Builder(context, ContractNotifier.CHANNEL)
            .setContentTitle(ContractNotifier.TITLE)
            .setContentText(channelMismatchHeadline(local))
            .setSmallIcon(android.R.drawable.stat_notify_error)
            .build())
        store.channelNotifiedAtMs = now()
    }

    companion object { const val CHANNEL_NOTIFICATION_ID = 6 }
}

/**
 * 更新を促す通知を端末に出す（D6）。専用のチャンネル `contract`。
 *
 * - 止まった状態に入った契機で 1 回、止まっている間は前に出してから 24 時間ごとに出し直す（同じ id なので欄には 1 通だけ）
 * - 前に出した時刻は [ContractWaitStore] に持つので、再起動しても 24 時間経つまで出し直さない
 * - 抜けたら通知を消し、画面の写しも消す
 * - 出せなかったとき（権限・チャンネルが切られている）は時刻を書かない。次の契機でまた試す
 */
class ContractNotifier(
    private val context: Context,
    private val store: ContractWaitStore,
    private val now: () -> Long = System::currentTimeMillis,
    private val log: (String) -> Unit = {},
) {
    private val manager: NotificationManager get() = context.getSystemService(NotificationManager::class.java)

    /**
     * 前の契機で出せなかったか。**出せない間は契機ごとにログを出さない**（final review R14）——
     * 通知を切った端末では 5 分ごとに同じ行が出続ける。出せない状態に入った契機で 1 行だけ残す
     */
    @Volatile
    private var skipping = false

    /** 送る契機の終わりに呼ぶ。[days] は手元に残している最も古い行を積んでからの日数。 */
    fun update(reasons: ContractReasons, days: Int) {
        val notice = contractNotice(reasons, days)
        // 画面が読む写し。通知が出せなくても画面には出す
        store.notice = notice
        if (notice == null) {
            if (store.notifiedAtMs != null) {
                manager.cancel(CONTRACT_NOTIFICATION_ID)
                store.notifiedAtMs = null
            }
            return
        }
        val last = store.notifiedAtMs
        if (last != null && now() - last < REPEAT.toMillis()) return
        if (post(notice)) store.notifiedAtMs = now()
    }

    private fun post(notice: ContractNotice): Boolean {
        if (!canPost()) {
            if (!skipping) log(Telemetry.line("contract_notice_skipped", source = null))
            skipping = true
            return false
        }
        skipping = false
        manager.createNotificationChannel(
            NotificationChannel(CHANNEL, "サーバの更新", NotificationManager.IMPORTANCE_DEFAULT),
        )
        manager.notify(
            CONTRACT_NOTIFICATION_ID,
            NotificationCompat.Builder(context, CHANNEL)
                .setContentTitle(TITLE)
                .setContentText(notice.line)
                .setSmallIcon(android.R.drawable.stat_notify_error)
                .build(),
        )
        return true
    }

    private fun canPost(): Boolean {
        val permitted = Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU ||
            context.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) == PackageManager.PERMISSION_GRANTED
        val channelOn = manager.getNotificationChannel(CHANNEL)?.importance != NotificationManager.IMPORTANCE_NONE
        return permitted && manager.areNotificationsEnabled() && channelOn
    }

    companion object {
        const val CHANNEL = "contract"

        /** 常駐（1）・保持（2）・途切れ（3）・写真と動画（4）と重ならない */
        const val CONTRACT_NOTIFICATION_ID = 5
        const val TITLE = "あしあと。"
        val REPEAT: Duration = Duration.ofHours(24)
    }
}
