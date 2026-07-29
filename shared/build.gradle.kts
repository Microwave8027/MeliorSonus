import org.jetbrains.kotlin.gradle.ExperimentalWasmDsl
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    alias(libs.plugins.kotlinMultiplatform)
    alias(libs.plugins.androidMultiplatformLibrary)
    alias(libs.plugins.composeMultiplatform)
    alias(libs.plugins.composeCompiler)
    alias(libs.plugins.kotlinxSerialization)
    alias(libs.plugins.wire)
    alias(libs.plugins.sqldelight)
    kotlin("native.cocoapods")
}

kotlin {
    listOf(
        iosArm64(),
        iosSimulatorArm64()
    ).forEach { iosTarget ->
        iosTarget.compilations.getByName("main") {
            // verovio_interop removed
        }
    }

    cocoapods {
        summary = "MeliorSonus Shared Library"
        homepage = "https://github.com/example/meliorsonus"
        version = "1.0"
        ios.deploymentTarget = "14.1"
        framework {
            baseName = "Shared"
            isStatic = true
        }

        // This ensures the .mm and .cpp files are compiled and linked by Xcode
        extraSpecAttributes["source_files"] = "'src/iosMain/objc/data_sources/**/*.{h,m,mm}'"
    }
    
    jvm()
    
    js {
        browser()
    }
    

    
    android {
       namespace = "com.example.meliorsonus.shared"
       compileSdk = libs.versions.android.compileSdk.get().toInt()
       minSdk = libs.versions.android.minSdk.get().toInt()
    
       compilerOptions {
           jvmTarget = JvmTarget.JVM_11
       }
       androidResources {
           enable = true
       }
       withHostTest {
           isIncludeAndroidResources = true
       }
    }
    
    sourceSets {
        commonMain {
            resources.srcDirs("src/commonMain/resources")
        }
        androidMain.dependencies {
            implementation(libs.compose.uiToolingPreview)
            implementation(libs.ktor.client.cio)
            implementation(libs.sqldelight.android.driver)
            implementation(libs.androidsvg)
            implementation(libs.koin.android)
        }

        val androidHostTest by getting {
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
        jsMain.dependencies {
            implementation(libs.wrappers.browser)
            implementation(libs.ktor.client.js)
        }
        iosMain.dependencies {
            implementation(libs.ktor.client.cio)
            implementation(libs.sqldelight.native.driver)
        }
        val iosMain by getting {
            resources.srcDirs("src/commonMain/webview")
        }
        jvmMain.dependencies {
            implementation(libs.ktor.client.cio)
            implementation(libs.sqldelight.sqlite.driver)
        }
        jvmTest.dependencies {
            implementation(libs.sqldelight.sqlite.driver)
        }
    }
}

dependencies {
    androidRuntimeClasspath(libs.compose.uiTooling)
}

wire {
    kotlin {
        // Wire generates kotlin classes from the proto schemas
    }
}

sqldelight {
    databases {
        create("MeliorSonusDatabase") {
            packageName.set("com.example.meliorsonus.db")
        }
    }
}

androidComponents {
    onVariants { variant ->
        variant.sources.assets?.addStaticSourceDirectory("src/commonMain/webview")
    }
}
