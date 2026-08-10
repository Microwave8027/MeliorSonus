package com.example.meliorsonus.ui.sheetviewer.core

import com.arkivanov.decompose.ComponentContext
import com.arkivanov.decompose.value.MutableValue
import com.arkivanov.decompose.value.Value
import com.arkivanov.decompose.value.update
import com.arkivanov.essenty.lifecycle.Lifecycle
import com.example.meliorsonus.model.SheetSearchResult
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import org.koin.core.component.KoinComponent

interface SheetViewerComponent {
    val state: Value<SheetViewerState>

    fun onBack()
    fun onTabSelected(tab: ViewerTab)
    fun onAiQueryChanged(query: String)

    enum class ViewerTab {
        FEEDBACK,
        STATISTICS,
        MODES,
        SUMMARY,
        METRONOME
    }

    data class SheetViewerState(
        val sheet: SheetSearchResult,
        val selectedTab: ViewerTab = ViewerTab.FEEDBACK,
        val aiQuery: String = "",
        val currentMeasure: Int = 1,
        val isLoading: Boolean = false,
        val errorMessage: String? = null,
        val totalPages: Int = 1,
    )
}

typealias ViewerTab = SheetViewerComponent.ViewerTab

class DefaultSheetViewerComponent(
    componentContext: ComponentContext,
    private val sheet: SheetSearchResult,
    private val onBack: () -> Unit,
) : SheetViewerComponent, KoinComponent, ComponentContext by componentContext {

    private val scope = CoroutineScope(Dispatchers.Main + SupervisorJob())

    private val _state = MutableValue(
        SheetViewerComponent.SheetViewerState(
            sheet = sheet,
            isLoading = false,
        )
    )
    override val state: Value<SheetViewerComponent.SheetViewerState> = _state

    init {
        lifecycle.subscribe(object : Lifecycle.Callbacks {
            override fun onDestroy() {
                scope.cancel()
            }
        })
    }

    override fun onBack() {
        onBack.invoke()
    }

    override fun onTabSelected(tab: ViewerTab) {
        _state.update { it.copy(selectedTab = tab) }
    }

    override fun onAiQueryChanged(query: String) {
        _state.update { it.copy(aiQuery = query) }
    }
}
