package com.example.meliorsonus.ui.home.core

import com.arkivanov.decompose.ComponentContext
import com.arkivanov.decompose.value.MutableValue
import com.arkivanov.decompose.value.Value
import com.arkivanov.decompose.value.update
import com.arkivanov.essenty.lifecycle.Lifecycle
import com.example.meliorsonus.domain.SaveAndDeletePDFUseCase
import com.example.meliorsonus.domain.SaveSheetUseCase
import com.example.meliorsonus.model.SheetSearchResult
import com.example.meliorsonus.repository.SavedSheetRepository
import com.example.meliorsonus.repository.SheetSearchRepository
import com.example.meliorsonus.ui.home.core.HomeComponent.HomeTab
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.TimeoutCancellationException
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeout
import org.koin.core.component.KoinComponent
import org.koin.core.component.inject

interface HomeComponent {

    val state: Value<HomeState>

    fun onTabSelected(tab: HomeTab)
    fun onLibrarySearchQueryChanged(query: String)
    fun onLibrarySearchSubmit()
    fun onOpenSheetViewer(sheet: SheetSearchResult)
    fun onOpenAddSheetPopup()
    fun onCloseAddSheetPopup()
    fun onSearchPopupQueryChanged(query: String)
    fun onSearchPopupInstrumentChanged(query: String)
    fun onSearchPopupSubmit()
    fun onSearchResultSelected(result: SheetSearchResult)
    fun onClearSearchSelection()
    fun onConfirmAndSaveSheet()
    fun onDeleteSheet(sheet: SheetSearchResult)
    fun onReturnToHome()
    fun loadMore()

    enum class HomeTab { HOME, SKILLS, LIBRARY, PROFILE }

    data class HomeState(
        val activeTab: HomeTab = HomeTab.HOME,
        val sheetMusicList: List<SheetSearchResult> = emptyList(),
        val librarySearchQuery: String = "",
        val isDrawerOpen: Boolean = false,

        // Add-sheet popup
        val showAddSheetPopup: Boolean = false,
        val searchPopupQuery: String = "",
        val searchPopupInstrument: String = "",
        val searchPopupResults: List<SheetSearchResult> = emptyList(),
        val isSearchPopupLoading: Boolean = false,
        val searchPopupError: String? = null,
        val selectedSearchResult: SheetSearchResult? = null,
        val pdfPath: String? = null,
        val isFetchingPdf: Boolean = false,
        val pdfFetchError: String? = null,
        val isFetchingMoreSheets: Boolean = false,
        val isSaving: Boolean = false,
        val pagesLoaded: Int = 0, // Pages Fetched
        val errorMessage: String? = null,
        val loadMoreError: String? = null
    )
}

class DefaultHomeComponent(
    componentContext: ComponentContext,
    private val onOpenSheetViewer: (SheetSearchResult) -> Unit
) : HomeComponent, KoinComponent, ComponentContext by componentContext {

    private val saveAndDeletePDFUseCase: SaveAndDeletePDFUseCase by inject()
    private val saveSheetUseCase: SaveSheetUseCase by inject()
    private val savedSheetRepository: SavedSheetRepository by inject()
    private val sheetSearchRepository: SheetSearchRepository by inject()

    private val scope = CoroutineScope(Dispatchers.Main + SupervisorJob())

    private val _state = MutableValue(HomeComponent.HomeState())
    override val state: Value<HomeComponent.HomeState> = _state

    init {
        lifecycle.subscribe(object : Lifecycle.Callbacks {
            override fun onResume() {
                loadSavedSheets()
            }

            override fun onDestroy() {
                scope.cancel()
            }
        })
    }

    private fun loadSavedSheets() {
        scope.launch {
            try {
                val saved = savedSheetRepository.getAllSavedSheets()
                _state.update { it.copy(
                    sheetMusicList = saved.map { s ->
                        SheetSearchResult(
                            mxl = s.mxl,
                            pdf = "",
                            title = s.title,
                            composer = s.composer,
                            songLengthBars = s.song_length_bars.toInt(),
                            genres = s.genres,
                            instruments = s.instruments.split(", ").filter { it.isNotBlank() }
                        )
                    }
                ) }
            } catch (e: Exception) {
                _state.update { it.copy(errorMessage = "Failed to load library: ${e.message}") }
            }
        }
    }

    override fun onTabSelected(tab: HomeTab) {
        _state.update { it.copy(activeTab = tab) }
        // Reload whenever the user returns to a data-driven tab
        if (tab == HomeTab.HOME || tab == HomeTab.LIBRARY) loadSavedSheets()
    }

    override fun onLibrarySearchQueryChanged(query: String) {
        _state.update { it.copy(librarySearchQuery = query) }
    }

    override fun onLibrarySearchSubmit() {
        // Filter locally for now; replace with a repo call if needed
        val query = _state.value.librarySearchQuery
        val all = _state.value.sheetMusicList
        _state.update {
            it.copy(
                sheetMusicList = if (query.isBlank()) all
                else all.filter { s ->
                    s.title.contains(query, ignoreCase = true) ||
                            s.composer.contains(query, ignoreCase = true)
                }
            )
        }
    }

    override fun onOpenSheetViewer(sheet: SheetSearchResult) {
        onOpenSheetViewer.invoke(sheet)
    }

    override fun onOpenAddSheetPopup() {
        _state.update {
            it.copy(
                showAddSheetPopup = true,
                searchPopupQuery = "",
                searchPopupInstrument = "",
                searchPopupResults = emptyList(),
                selectedSearchResult = null,
                pdfPath = null,
                searchPopupError = null,
                pdfFetchError = null,
                isSaving = false
            )
        }
    }

    private fun cleanupPdf() {
        _state.value.pdfPath?.let { path ->
            scope.launch {
                saveAndDeletePDFUseCase(path)
                _state.update { it.copy(pdfPath = null) }
            }
        }
    }

    override fun onCloseAddSheetPopup() {
        cleanupPdf()
        _state.update { it.copy(showAddSheetPopup = false) }
    }

    override fun onSearchPopupQueryChanged(query: String) {
        _state.update { it.copy(searchPopupQuery = query) }
    }

    override fun onSearchPopupInstrumentChanged(query: String) {
        _state.update { it.copy(searchPopupInstrument = query) }
    }

    private suspend fun searchSheets(query: String, pages: Int = 10, instrument: String = ""): List<SheetSearchResult>  =
        withTimeout(3000L) { sheetSearchRepository.searchSheets(query, pages, instrument) }


    override fun onSearchPopupSubmit() {
        val query = _state.value.searchPopupQuery
        val instrument = _state.value.searchPopupInstrument
        scope.launch {
            _state.update { it.copy(isSearchPopupLoading = true, searchPopupError = null, searchPopupResults = emptyList()) }
            try {
                val results = searchSheets(query, instrument = instrument)
                _state.update { it.copy(searchPopupResults = results, pagesLoaded = 10) }
            } catch (e: TimeoutCancellationException) {
                _state.update { it.copy(searchPopupError = "Search timed out. Please try again.") }
            } catch (e: Exception) {
                _state.update { it.copy(searchPopupError = "Search failed: ${e.message ?: "Unknown error"}") }
            } finally {
                _state.update { it.copy(isSearchPopupLoading = false) }
            }
        }
    }

    override fun onSearchResultSelected(result: SheetSearchResult) {
        cleanupPdf()
        _state.update { it.copy(selectedSearchResult = result, pdfPath = null, pdfFetchError = null) }
        scope.launch {
            _state.update { it.copy(isFetchingPdf = true) }
            try {
                val path = withTimeout(5000L) { sheetSearchRepository.fetchPdf(result.pdf) }
                _state.update { it.copy(pdfPath = path) }
            } catch (e: TimeoutCancellationException) {
                _state.update { it.copy(pdfFetchError = "PDF load timed out. Please try again.") }
            } catch (e: Exception) {
                _state.update { it.copy(pdfFetchError = "Failed to load PDF: ${e.message ?: "Unknown error"}") }
            } finally {
                _state.update { it.copy(isFetchingPdf = false) }
            }
        }
    }

    override fun onClearSearchSelection() {
        cleanupPdf()
        _state.update { it.copy(selectedSearchResult = null, pdfFetchError = null) }
    }

    override fun onConfirmAndSaveSheet() {
        val result = _state.value.selectedSearchResult ?: return
        _state.update { it.copy(isSaving = true, searchPopupError = null) }
        scope.launch {
            try {
                val xmlLocalPath = sheetSearchRepository.fetchMXL(result.mxl)
                val resultWithLocalMxl = result.copy(mxl = xmlLocalPath)
                val savedMxl = saveSheetUseCase(resultWithLocalMxl)
                cleanupPdf()
                loadSavedSheets()
                _state.update { it.copy(
                    showAddSheetPopup = false,
                    isSaving = false,
                    searchPopupQuery = "",
                    searchPopupInstrument = "",
                    searchPopupResults = emptyList(),
                    selectedSearchResult = null,
                    pdfPath = null
                ) }
                onOpenSheetViewer(resultWithLocalMxl.copy(mxl = savedMxl))
            } catch (e: Exception) {
                _state.update { it.copy(isSaving = false, searchPopupError = "Failed to save: ${e.message ?: "Unknown error"}") }
            }
        }
    }

    override fun onDeleteSheet(sheet: SheetSearchResult) {
        scope.launch {
            try {
                savedSheetRepository.deleteSheet(sheet.mxl)
                loadSavedSheets()
            } catch (e: Exception) {
                _state.update { it.copy(errorMessage = "Failed to delete: ${e.message}") }
            }
        }
    }

    override fun onReturnToHome() {
        _state.update { it.copy(activeTab = HomeTab.HOME) }
    }

    override fun loadMore() {
        if (_state.value.isFetchingMoreSheets) return
        scope.launch {
            _state.update { it.copy(isFetchingMoreSheets = true, loadMoreError = null) }
            try {
                val query = _state.value.searchPopupQuery
                val instrument = _state.value.searchPopupInstrument
                val nextPage = _state.value.pagesLoaded + 10
                val results = searchSheets(query, pages = nextPage, instrument = instrument)
                _state.update { it.copy(
                    searchPopupResults = results,
                    pagesLoaded = nextPage
                ) }
            } catch (e: Exception) {
                _state.update { it.copy(loadMoreError = "Failed to load more: ${e.message}") }
            } finally {
                _state.update { it.copy(isFetchingMoreSheets = false) }
            }
        }
    }
}
