// SPDX-License-Identifier: AGPL-3.0-only
plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "dev.ashiato.collector"
    compileSdk = 36
    defaultConfig {
        applicationId = "dev.ashiato.collector"
        minSdk = 30          // Health Connect と scoped storage の前提
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"
    }
    kotlinOptions { jvmTarget = "17" }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

dependencies {
    implementation("androidx.core:core-ktx:1.17.0")
}
