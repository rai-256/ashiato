// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
pluginManagement { repositories { google(); mavenCentral(); gradlePluginPortal() } }
val extraRepo = gradle.startParameter.projectProperties["ashiato.extraRepo"]
dependencyResolutionManagement {
    repositories {
        if (!extraRepo.isNullOrBlank()) maven { url = uri(extraRepo) }
        google()
        mavenCentral()
    }
}
rootProject.name = "ashiato-collector-android"
include(":app")
