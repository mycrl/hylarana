import com.android.build.gradle.internal.tasks.factory.dependsOn
import java.util.Locale

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.compose)

    kotlin("plugin.serialization") version libs.versions.kotlin.get()
}

android {
    namespace = "com.example.hylarana.app"
    compileSdk = 35

    defaultConfig {
        applicationId = "com.example.hylarana.app"
        minSdk = 29
        targetSdk = 35
        versionCode = 1
        versionName = "1.0"
    }

    buildTypes {
        release {
            isDebuggable = false
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro"
            )
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }

    kotlinOptions {
        jvmTarget = "11"
    }

    buildFeatures {
        compose = true
        buildConfig = true
    }
}

dependencies {

    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.lifecycle.runtime.ktx)
    implementation(libs.androidx.activity.compose)
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.ui)
    implementation(libs.androidx.ui.graphics)
    implementation(libs.androidx.ui.tooling.preview)
    implementation(libs.androidx.material3)
    implementation(libs.androidx.webkit)
    implementation(libs.kotlinx.serialization.json)
    implementation(libs.kotlin.faker)
    implementation(project(":hylarana"))
}

val runPreCopyCommand by tasks.registering(Exec::class) {
    workingDir = file("../../frontend")

    doFirst {
        if (System.getProperty("os.name").lowercase(Locale.getDefault()).contains("win")) {
            commandLine("powershell", "-Command", "yarn build")
        } else {
            commandLine("bash", "-c", "yarn build")
        }
    }
}

val copyAssets by tasks.registering(Copy::class) {
    from("../../frontend/dist")
    into("src/main/assets")
    include("**/*")
}

copyAssets.dependsOn(runPreCopyCommand)

tasks.named("preBuild") {
    dependsOn(copyAssets)
}