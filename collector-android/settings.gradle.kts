// SPDX-License-Identifier: AGPL-3.0-only
pluginManagement { repositories { google(); mavenCentral(); gradlePluginPortal() } }
dependencyResolutionManagement { repositories { google(); mavenCentral() } }
rootProject.name = "ashiato-collector-android"
include(":app")
