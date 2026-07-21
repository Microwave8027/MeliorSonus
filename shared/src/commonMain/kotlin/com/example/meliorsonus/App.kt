package com.example.meliorsonus

import androidx.compose.runtime.Composable
import androidx.compose.ui.tooling.preview.Preview
import com.arkivanov.decompose.DefaultComponentContext
import com.arkivanov.essenty.lifecycle.LifecycleRegistry
import com.example.meliorsonus.theme.Miscellaneous.GlobalMaterialTheme
import com.example.meliorsonus.ui.root.DefaultRootComponent
import com.example.meliorsonus.ui.root.RootScreen

@Composable
fun App(rootComponent: DefaultRootComponent) {
    GlobalMaterialTheme {
        RootScreen(component = rootComponent)
    }
}

@Preview
@Composable
private fun AppPreview() {
    val lifecycle = LifecycleRegistry()
    val root = DefaultRootComponent(DefaultComponentContext(lifecycle))
    App(rootComponent = root)
}