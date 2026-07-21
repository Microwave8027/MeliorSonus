package com.example.meliorsonus

import androidx.compose.ui.window.Window
import androidx.compose.ui.window.application
import com.arkivanov.decompose.DefaultComponentContext
import com.arkivanov.essenty.lifecycle.LifecycleRegistry
import com.example.meliorsonus.ui.root.DefaultRootComponent

fun main() {
    val lifecycle = LifecycleRegistry()
    val rootComponent = DefaultRootComponent(DefaultComponentContext(lifecycle))

    application {
        Window(
            onCloseRequest = ::exitApplication,
            title = "MeliorSonus",
        ) {
            App(rootComponent = rootComponent)
        }
    }
}