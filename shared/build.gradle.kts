

plugins {
    alias(libs.plugins.kotlinMultiplatform)
    alias(libs.plugins.androidLibrary)
    alias(libs.plugins.composeMultiplatform)
    alias(libs.plugins.composeCompiler)
    alias(libs.plugins.kotlinxSerialization)
    alias(libs.plugins.wire)
    alias(libs.plugins.sqldelight)
    id("dev.gobley.cargo")
    id("dev.gobley.uniffi")
    id("org.jetbrains.kotlin.plugin.atomicfu")
}

android {
    namespace = "com.example.meliorsonus.shared"
    compileSdk = libs.versions.android.compileSdk.get().toInt()
    ndkVersion = libs.versions.ndk.get()

    defaultConfig {
        minSdk = libs.versions.android.minSdk.get().toInt()
    }

    androidResources {
        enable = true
    }
    testOptions {
        unitTests {
            isIncludeAndroidResources = true
        }
    }
}

val isMac = org.apache.tools.ant.taskdefs.condition.Os.isFamily(
    org.apache.tools.ant.taskdefs.condition.Os.FAMILY_MAC
)

kotlin {
    androidTarget()

    if (isMac) {
        listOf(
            iosArm64(),
            iosSimulatorArm64()
        ).forEach { iosTarget ->
            iosTarget.binaries.framework {
                baseName = "Shared"
                isStatic = true

                export(libs.decompose)
                export(libs.essenty.lifecycle)
            }
        }

    }
    sourceSets {
        commonMain {
            kotlin.srcDir("build/generated/source/wire/commonMain")
            resources.srcDirs("src/commonMain/resources")
        }
        androidMain.dependencies {
            implementation(libs.compose.uiToolingPreview)
            implementation(libs.ktor.client.cio)
            implementation(libs.sqldelight.android.driver)
            implementation(libs.androidsvg)
            implementation(libs.koin.android)
            implementation("net.java.dev.jna:jna:5.14.0@aar")
        }

        val androidUnitTest by getting {
            dependencies {
                implementation(libs.sqldelight.sqlite.driver)
            }
        }
        commonMain.dependencies {
            implementation(libs.compose.runtime)
            implementation(libs.compose.foundation)
            implementation(libs.compose.material3)
            implementation(libs.compose.material3.windowSizeClass)
            implementation(compose.materialIconsExtended)
            implementation(libs.compose.ui)
            implementation(libs.compose.components.resources)
            implementation(libs.compose.uiToolingPreview)
            implementation(libs.compose.preview)
            implementation(libs.androidx.lifecycle.viewmodelCompose)
            implementation(libs.androidx.lifecycle.runtimeCompose)
            
            implementation(libs.kotlinxCoroutinesCore)

            // Koin DI
            api(libs.koin.core)
            implementation(libs.koin.compose)
            
            // Decompose
            api(libs.decompose)
            implementation(libs.decompose.compose)
            api(libs.essenty.lifecycle)
            
            // Ktor network
            implementation(libs.ktor.client.core)
            implementation(libs.ktor.client.content.negotiation)
            implementation(libs.ktor.serialization.kotlinx.json)
            implementation(libs.kotlinx.serialization.json.okio)
            implementation(libs.ktor.client.mock)
            
            // Wire Proto, Okio & DataStore
            implementation(libs.wire.runtime)
            implementation(libs.okio)
            implementation(libs.androidx.datastore.core)
            
            // SQLDelight
            implementation(libs.sqldelight.runtime)
            implementation(libs.sqldelight.coroutines.extensions)

            //Coil for SVG rendering
            implementation(libs.coil.compose)
            implementation(libs.coil.network.ktor)
            implementation(libs.coil.svg)
        }
        commonTest.dependencies {
            implementation(libs.kotlin.test)
            implementation(libs.kotlinx.coroutines.test)
        }
        iosMain.dependencies {
            implementation(libs.ktor.client.darwin)
            implementation(libs.sqldelight.native.driver)
        }
    }
}

wire {
    sourcePath {
        srcDir("src/commonMain/proto")
    }
    kotlin {
        out = "build/generated/source/wire/commonMain"
        rpcRole = "none"
    }
}

sqldelight {
    databases {
        create("MeliorSonusDatabase") {
            packageName.set("com.example.meliorsonus.db")
            verifyMigrations.set(false)
        }
    }
}

androidComponents {
    onVariants { variant ->
        variant.sources.assets?.addStaticSourceDirectory("src/commonMain/webview")
    }
}

tasks.withType<org.jetbrains.kotlin.gradle.tasks.KotlinCompilationTask<*>>().configureEach {
    dependsOn(tasks.matching { it.name.endsWith("Protos", ignoreCase = true) })
}

tasks.matching {
    it.name.contains("Ios", ignoreCase = true) &&
    (it.name.contains("cargo", ignoreCase = true) ||
     it.name.contains("rust", ignoreCase = true) ||
     it.name.contains("uniffi", ignoreCase = true) ||
     it.name.contains("cinterop", ignoreCase = true))
}.configureEach {
    enabled = false
}

tasks.matching { it.name.contains("verify", ignoreCase = true) && it.name.contains("Migration", ignoreCase = true) }.configureEach {
    enabled = false
}

tasks.register("sync") {
    group = "ide"
    description = "Explicit root sync task to prevent Gradle task abbreviation ambiguity."

    val sharedProject = evaluationDependsOn(":shared")
    dependsOn(sharedProject.tasks.matching {
        (it.name.startsWith("cargo") || it.name.contains("Uniffi")) &&
        !it.name.contains("Ios", ignoreCase = true) &&
        !it.name.contains("Clean", ignoreCase = true)
    })
}


