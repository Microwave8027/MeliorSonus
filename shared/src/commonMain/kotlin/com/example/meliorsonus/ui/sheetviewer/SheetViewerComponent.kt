package com.example.meliorsonus.ui.sheetviewer

import androidx.lifecycle.viewmodel.compose.viewModel
import com.arkivanov.decompose.ComponentContext
import com.arkivanov.decompose.value.MutableValue
import com.arkivanov.decompose.value.Value
import com.arkivanov.decompose.value.update
import com.example.meliorsonus.model.SheetSearchResult
import com.example.meliorsonus.repository.SavedSheetRepository
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import org.koin.core.component.KoinComponent
import org.koin.core.component.inject

interface SheetViewerComponent {
    val state: Value<SheetViewerState>

    fun onBack()
    fun onZoomIn()
    fun onZoomOut()
    fun onNextPage()
    fun onPreviousPage()
    fun fetchMxl()
    fun updateDrag(isDragging: Boolean)

    data class SheetViewerState(
        val sheet: SheetSearchResult,
        val zoom: Float = 1.0f,
        val currentMeasure: Int = 1,
        /**
         * Monotonically incrementing counters used as one-shot events.
         * Each increment triggers a LaunchedEffect in the WebView that
         * calls goToNextPage() / goToPreviousPage() in JavaScript.
         */
        val pageNextCount: Int = 0,
        val pagePreviousCount: Int = 0,
        val mxl: String = "",
        val isLoading: Boolean = true,
        val isDragging: Boolean = false
    )
}

class DefaultSheetViewerComponent(
    componentContext: ComponentContext,
    private val sheet: SheetSearchResult,
    private val onBack: () -> Unit
) : SheetViewerComponent, KoinComponent, ComponentContext by componentContext {

    private val scope = CoroutineScope(Dispatchers.Main + SupervisorJob())

    private val savedSheetRepository: SavedSheetRepository by inject()
    private val _state = MutableValue(SheetViewerComponent.SheetViewerState(sheet = sheet))
    override val state: Value<SheetViewerComponent.SheetViewerState> = _state

    init {
        lifecycle.subscribe(object : com.arkivanov.essenty.lifecycle.Lifecycle.Callbacks {
            override fun onCreate() {
                _state.update{it.copy(isLoading = true)}
            }
            override fun onDestroy() {
                scope.cancel()
            }
        })
    }


    override fun fetchMxl() {
        val fileName = sheet.mxl.substringAfterLast("/").substringAfterLast("\\")
        scope.launch {
            _state.update { it.copy(isLoading = true) }
            try {
                val content = savedSheetRepository.getMXL(fileName)
                _state.update { it.copy(mxl = content, isLoading = false) }
            } catch (_: Exception) {
                // If the file is missing (e.g. deletion failed partway or manual deletion)
                // we stop loading but keep mxl empty. Screen should probably show an error.
                _state.update { it.copy(isLoading = false, mxl = "") }
            }
        }
    }
    override fun onBack() = onBack.invoke()

    override fun onZoomIn() {
        _state.update { it.copy(zoom = (it.zoom + 0.1f).coerceIn(0.5f, 2.5f)) }
    }

    override fun onZoomOut() {
        _state.update { it.copy(zoom = (it.zoom - 0.1f).coerceIn(0.5f, 2.5f)) }
    }

    override fun onNextPage() {
        _state.update { it.copy(pageNextCount = it.pageNextCount + 1) }
    }

    override fun onPreviousPage() {
        _state.update { it.copy(pagePreviousCount = it.pagePreviousCount + 1) }
    }

    override fun updateDrag(isDragging: Boolean) {
        _state.update { it.copy(isDragging = isDragging) }
    }
}
