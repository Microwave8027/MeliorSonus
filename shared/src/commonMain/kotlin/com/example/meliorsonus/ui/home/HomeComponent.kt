package com.example.meliorsonus.ui.home

import com.arkivanov.decompose.ComponentContext
import com.arkivanov.decompose.value.MutableValue
import com.arkivanov.decompose.value.Value
import com.arkivanov.decompose.value.update
import com.example.meliorsonus.domain.SaveSheetUseCase
import com.example.meliorsonus.model.SheetSearchResult
import com.example.meliorsonus.repository.SavedSheetRepository
import com.example.meliorsonus.repository.SheetSearchRepository
import com.example.meliorsonus.ui.home.HomeComponent.HomeTab
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
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
    fun onSearchPopupSubmit()
    fun onSearchResultSelected(result: SheetSearchResult)
    fun onClearSearchSelection()
    fun onConfirmAndSaveSheet()
    fun onDeleteSheet(sheet: SheetSearchResult)

    enum class HomeTab { HOME, SKILLS, LIBRARY, PROFILE }

    data class HomeState(
        val activeTab: HomeTab = HomeTab.HOME,
        val sheetMusicList: List<SheetSearchResult> = emptyList(),
        val librarySearchQuery: String = "",
        val isDrawerOpen: Boolean = false,

        // Add-sheet popup
        val showAddSheetPopup: Boolean = false,
        val searchPopupQuery: String = "",
        val searchPopupResults: List<SheetSearchResult> = emptyList(),
        val isSearchPopupLoading: Boolean = false,
        val searchPopupError: String? = null,
        val selectedSearchResult: SheetSearchResult? = null,
        val pdfPath: String? = null,
        val isFetchingPdf: Boolean = false,
        val pdfFetchError: String? = null,

        val isSaving: Boolean = false,

        val errorMessage: String? = null
    )
}

class DefaultHomeComponent(
    componentContext: ComponentContext,
    private val onOpenSheetViewer: (SheetSearchResult) -> Unit
) : HomeComponent, KoinComponent, ComponentContext by componentContext {

    private val saveSheetUseCase: SaveSheetUseCase by inject()
    private val savedSheetRepository: SavedSheetRepository by inject()
    private val sheetSearchRepository: SheetSearchRepository by inject()

    private val scope = CoroutineScope(Dispatchers.Main + SupervisorJob())

    private val _state = MutableValue(HomeComponent.HomeState())
    override val state: Value<HomeComponent.HomeState> = _state

    init {
        lifecycle.subscribe(object : com.arkivanov.essenty.lifecycle.Lifecycle.Callbacks {
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
                            artistName = s.artist_name,
                            composerName = s.composer_name,
                            publisher = s.publisher,
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
                            s.composerName.contains(query, ignoreCase = true) ||
                            s.artistName.contains(query, ignoreCase = true)
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
                sheetSearchRepository.deletePdf(path)
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

    override fun onSearchPopupSubmit() {
        val query = _state.value.searchPopupQuery
        scope.launch {
            _state.update { it.copy(isSearchPopupLoading = true, searchPopupError = null, searchPopupResults = emptyList()) }
            try {
                val results = withTimeout(3000L) { sheetSearchRepository.searchSheets(query) }
                _state.update { it.copy(searchPopupResults = results) }
            } catch (e: kotlinx.coroutines.TimeoutCancellationException) {
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
            } catch (e: kotlinx.coroutines.TimeoutCancellationException) {
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
                _state.update { it.copy(showAddSheetPopup = false, isSaving = false) }
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
}

