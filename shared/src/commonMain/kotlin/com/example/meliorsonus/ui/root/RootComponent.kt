package com.example.meliorsonus.ui.root

import com.arkivanov.decompose.ComponentContext
import com.arkivanov.decompose.router.stack.ChildStack
import com.arkivanov.decompose.router.stack.StackNavigation
import com.arkivanov.decompose.router.stack.childStack
import com.arkivanov.decompose.router.stack.pop
import com.arkivanov.decompose.router.stack.push
import com.arkivanov.decompose.value.Value
import com.example.meliorsonus.model.SheetSearchResult
import com.example.meliorsonus.ui.home.DefaultHomeComponent
import com.example.meliorsonus.ui.home.HomeComponent
import com.example.meliorsonus.ui.sheetviewer.DefaultSheetViewerComponent
import com.example.meliorsonus.ui.sheetviewer.SheetViewerComponent
import com.example.meliorsonus.util.SizeOfScreen
import kotlinx.serialization.Serializable
import org.koin.core.component.KoinComponent
import org.koin.core.component.inject

interface RootComponent {
    val stack: Value<ChildStack<*, Child>>

    sealed class Child {
        class HomeChild(val component: HomeComponent) : Child()
        class SheetViewerChild(val component: SheetViewerComponent) : Child()
    }
}

class DefaultRootComponent(
    componentContext: ComponentContext
) : RootComponent, KoinComponent, ComponentContext by componentContext {

    private val navigation = StackNavigation<Config>()

    private val sizeOfScreen: SizeOfScreen by inject()

    override val stack: Value<ChildStack<*, RootComponent.Child>> = childStack(
        source = navigation,
        serializer = Config.serializer(),
        initialConfiguration = Config.Home,
        handleBackButton = true,
        childFactory = ::createChild
    )

    private fun createChild(config: Config, context: ComponentContext): RootComponent.Child =
        when (config) {
            is Config.Home -> RootComponent.Child.HomeChild(
                DefaultHomeComponent(
                    componentContext = context,
                    onOpenSheetViewer = { sheet ->
                        navigation.push(Config.SheetViewer(sheet))
                    }
                )
            )
            is Config.SheetViewer -> RootComponent.Child.SheetViewerChild(
                DefaultSheetViewerComponent(
                    componentContext = context,
                    sheet = config.sheet,
                    onBack = { navigation.pop() },
                    sizeOfScreen = sizeOfScreen
                )
            )
        }

    @Serializable
    private sealed class Config {
        @Serializable data object Home : Config()
        @Serializable data class SheetViewer(val sheet: SheetSearchResult) : Config()
    }
}
