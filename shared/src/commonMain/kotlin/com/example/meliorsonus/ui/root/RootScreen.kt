package com.example.meliorsonus.ui.root

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import com.arkivanov.decompose.extensions.compose.stack.Children
import com.arkivanov.decompose.extensions.compose.stack.animation.slide
import com.arkivanov.decompose.extensions.compose.stack.animation.stackAnimation
import com.example.meliorsonus.ui.home.HomeScreen
import com.example.meliorsonus.ui.sheetviewer.SheetViewerScreen

@Composable
fun RootScreen(
    component: RootComponent,
    modifier: Modifier = Modifier
) {
    Children(
        stack = component.stack,
        modifier = modifier,
        animation = stackAnimation(slide())
    ) { child ->
        when (val instance = child.instance) {
            is RootComponent.Child.HomeChild -> HomeScreen(component = instance.component)
            is RootComponent.Child.SheetViewerChild -> SheetViewerScreen(component = instance.component)
        }
    }
}
