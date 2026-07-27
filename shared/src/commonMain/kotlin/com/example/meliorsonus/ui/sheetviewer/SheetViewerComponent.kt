package com.example.meliorsonus.ui.sheetviewer

import androidx.lifecycle.viewmodel.compose.viewModel
import com.arkivanov.decompose.ComponentContext
import com.arkivanov.decompose.childContext
import com.arkivanov.decompose.value.MutableValue
import com.arkivanov.decompose.value.Value
import com.arkivanov.decompose.value.update
import com.example.meliorsonus.domain.GetSVGUseCase
import com.example.meliorsonus.domain.SaveSheetUseCase
import com.example.meliorsonus.model.ScoreMetadata
import com.example.meliorsonus.model.SheetSearchResult
import com.example.meliorsonus.repository.SavedSheetRepository
import com.example.meliorsonus.ui.home.DefaultVerovioManagerComponent
import com.example.meliorsonus.ui.home.VerovioManagerComponent
import com.example.meliorsonus.util.SizeOfScreen
import com.example.meliorsonus.util.appFilesDir
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import kotlinx.serialization.json.JsonNull.content
import okio.Path
import okio.Path.Companion.toPath
import org.koin.core.component.KoinComponent
import org.koin.core.component.inject

interface SheetViewerComponent {
    val state: Value<SheetViewerState>
    val verovioManagerComponent: VerovioManagerComponent

    fun onBack()
    fun onNextPage()
    fun onPreviousPage()
    fun fetchSVG()
    fun resizeDone()
    fun onZoomLevelChanged(zoom: Float)
    fun onConfirmResize()
    fun onCancelResize()

    data class SheetViewerState(
        val sheet: SheetSearchResult,
        val currentMeasure: Int = 1,
        /**
         * Monotonically incrementing counters used as one-shot events.
         * Each increment triggers a LaunchedEffect in the WebView that
         * calls goToNextPage() / goToPreviousPage() in JavaScript.
         */
        val pageNextCount: Int = 0,
        val pagePreviousCount: Int = 0,
        val svgPath: String = "",
        val svgMetaData: List<ScoreMetadata> = emptyList(),
        val isLoading: Boolean = true,
        val isTablet: Boolean = false,
        val shouldResize: Boolean = false,
        val zoomLevel: Float = 1.0f,
        val totalPages: Int = 1,
    )
}



class DefaultSheetViewerComponent(
    componentContext: ComponentContext,
    private val sheet: SheetSearchResult,
    private val onBack: () -> Unit,
    private val sizeOfScreen: SizeOfScreen
) : SheetViewerComponent, KoinComponent, ComponentContext by componentContext {

    private val scope = CoroutineScope(Dispatchers.Main + SupervisorJob())
    private val getSVGUseCase: GetSVGUseCase by inject()
    private val saveSheetUseCase: SaveSheetUseCase by inject()

    override val verovioManagerComponent: VerovioManagerComponent = DefaultVerovioManagerComponent(childContext("VerovioManager"))

    private val _state = MutableValue(SheetViewerComponent.SheetViewerState(sheet = sheet, isLoading = true, isTablet = if(minOf(sizeOfScreen.getSize().first, sizeOfScreen.getSize().second) > 600) true else false))
    override val state: Value<SheetViewerComponent.SheetViewerState> = _state

    init {
        lifecycle.subscribe(object : com.arkivanov.essenty.lifecycle.Lifecycle.Callbacks {
            override fun onDestroy() {
                scope.cancel()
            }
        })
    }


    override fun fetchSVG() {
        val fileName = sheet.mxl.replace("\\", "/").substringAfterLast("/")
        scope.launch {
            _state.update { it.copy(isLoading = true) }
            try {
                val content = getSVGUseCase(fileName, if(state.value.isTablet)"tablet" else "phone")
                if (content.isError){
                    _state.update{ it.copy(shouldResize = true, isLoading = false)}
                    verovioManagerComponent.loadMxl(sheet.mxl)
                } else {
                    _state.update { it.copy(svgPath = content.pathToSVG, svgMetaData = content.metaData, isLoading = false, totalPages = content.totalPages) }
                }
            } catch (_: Exception) {
                _state.update { it.copy(isLoading = false, svgPath = "", svgMetaData = emptyList()) }
            }
        }
    }

    override fun resizeDone(){
        _state.update{it.copy(shouldResize = false)}
    }

    override fun onBack() {
        verovioManagerComponent.cleanup()
        onBack.invoke()
    }


    override fun onNextPage() {
        _state.update { it.copy(pageNextCount = it.pageNextCount + 1) }
    }

    override fun onPreviousPage() {
        _state.update { it.copy(pagePreviousCount = it.pagePreviousCount + 1) }
    }

    override fun onZoomLevelChanged(zoom: Float) {
        _state.update { it.copy(zoomLevel = zoom) }
    }

    override fun onConfirmResize() {
        _state.update { it.copy(shouldResize = false, isLoading = true) }
        verovioManagerComponent.cleanup()
        scope.launch {
            try {
                saveSheetUseCase(sheet, _state.value.zoomLevel)
                fetchSVG()
            } catch (_: Exception) {
                _state.update { it.copy(isLoading = false) }
            }
        }
    }

    override fun onCancelResize() {
        verovioManagerComponent.cleanup()
        _state.update { it.copy(shouldResize = false) }
        onBack()
    }

}

