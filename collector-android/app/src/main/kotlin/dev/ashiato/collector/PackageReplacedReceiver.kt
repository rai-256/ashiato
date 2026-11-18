// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
package dev.ashiato.collector

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import androidx.core.content.ContextCompat
import java.io.File

class PackageReplacedReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != Intent.ACTION_MY_PACKAGE_REPLACED) return
        Config.initialize(context)
        if (!Config.isComplete) return
        val marker = File(context.noBackupFilesDir, "update/replaced-start")
        marker.parentFile?.mkdirs()
        marker.writeText("1")
        ContextCompat.startForegroundService(context, Intent(context, LocationService::class.java))
    }
}
