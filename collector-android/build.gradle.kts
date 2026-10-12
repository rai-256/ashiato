// SPDX-License-Identifier: AGPL-3.0-only
plugins {
    id("com.android.application") version "8.13.0" apply false
    id("org.jetbrains.kotlin.android") version "2.2.20" apply false
    id("org.jetbrains.kotlin.plugin.serialization") version "2.2.20" apply false
    // lint（2026-10-11）。ktlint は detekt の formatting ルールセット（detekt-formatting）で回す ——
    // 道具も baseline も 1 つで済む。設定は config/detekt.yml、凍結した既存の違反は config/detekt-baseline.xml
    id("io.gitlab.arturbosch.detekt") version "1.23.8" apply false
}
