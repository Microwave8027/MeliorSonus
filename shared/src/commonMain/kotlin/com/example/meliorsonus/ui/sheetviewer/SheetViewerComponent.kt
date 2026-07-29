package com.example.meliorsonus.ui.sheetviewer

import com.arkivanov.decompose.ComponentContext
import com.arkivanov.decompose.value.MutableValue
import com.arkivanov.decompose.value.Value
import com.example.meliorsonus.model.SheetSearchResult
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import org.koin.core.component.KoinComponent

interface SheetViewerComponent {
    val state: Value<SheetViewerState>

    fun onBack()
    fun onNextPage()
    fun onPreviousPage()

    data class SheetViewerState(
        val sheet: SheetSearchResult,
        val currentMeasure: Int = 1,
        val isLoading: Boolean = false,
        val errorMessage: String? = null,
        val totalPages: Int = 1,
    )
}

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
        lifecycle.subscribe(object : com.arkivanov.essenty.lifecycle.Lifecycle.Callbacks {
            override fun onDestroy() {
                scope.cancel()
            }
        })
    }

    override fun onBack() {
        onBack.invoke()
    }

    override fun onNextPage() {
    }

    override fun onPreviousPage() {
    }
}
