// SPDX-License-Identifier: AGPL-3.0-only
plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.serialization")
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

        // 接続先と資格情報は**コミットしない**（製造準備 A-2）。
        // ~/.gradle/gradle.properties か -P で渡す。既定は空で、空のまま動かすと送信は行われない。
        buildConfigField("String", "BASE_URL",
            "\"${project.findProperty("ashiato.baseUrl") ?: ""}\"")
        buildConfigField("String", "API_TOKEN",
            "\"${project.findProperty("ashiato.apiToken") ?: ""}\"")
        buildConfigField("String", "USER_ID",
            "\"${project.findProperty("ashiato.userId") ?: ""}\"")
    }
    buildFeatures { buildConfig = true }
    // Robolectric は本物の framework を JVM 上で動かすので資源が要る。
    // **本番経路（LocationCallback / Activity の権限フロー）を実機なしで通すための唯一の道具**
    testOptions { unitTests { isIncludeAndroidResources = true } }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

// Kotlin 2.x の DSL。kotlinOptions { jvmTarget } は非推奨。
kotlin {
    compilerOptions {
        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17)
    }
}

dependencies {
    implementation("androidx.core:core-ktx:1.17.0")
    implementation("com.google.android.gms:play-services-location:21.3.0")
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.9.0")
    testImplementation("junit:junit:4.13.2")
    testImplementation("org.robolectric:robolectric:4.16")
    testImplementation("androidx.test:core:1.7.0")
}
