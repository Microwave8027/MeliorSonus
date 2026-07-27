package com.example.meliorsonus

import androidx.compose.runtime.Composable
import androidx.compose.ui.tooling.preview.Preview
import coil3.ImageLoader
import coil3.compose.setSingletonImageLoaderFactory
import com.arkivanov.decompose.DefaultComponentContext
import com.arkivanov.essenty.lifecycle.LifecycleRegistry
import com.example.meliorsonus.theme.Miscellaneous.GlobalMaterialTheme
import com.example.meliorsonus.ui.root.DefaultRootComponent
import com.example.meliorsonus.ui.root.RootScreen
import org.koin.compose.koinInject

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

    val imageLoader = koinInject<ImageLoader>()
    setSingletonImageLoaderFactory { imageLoader }

    val root = DefaultRootComponent(DefaultComponentContext(lifecycle))
    App(rootComponent = root)
}