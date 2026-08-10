package com.example.meliorsonus.ui.home.tabDirectories

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import com.example.meliorsonus.model.SheetSearchResult
import com.example.meliorsonus.ui.home.addSheet.AddSheetPopupContent

@Composable
fun LibraryContent(
    isTablet: Boolean,
    query: String,
    onQueryChanged: (String) -> Unit,
    instrumentQuery: String,
    onInstrumentQueryChanged: (String) -> Unit,
    onSearchSubmit: () -> Unit,
    isLoading: Boolean,
    searchError: String?,
    results: List<SheetSearchResult>,
    selectedResult: SheetSearchResult?,
    pdfPath: String?,
    isFetchingPdf: Boolean,
    pdfFetchError: String?,
    isSaving: Boolean,
    onResultClick: (SheetSearchResult) -> Unit,
    onClearSelection: () -> Unit,
    onConfirmAndSave: () -> Unit,
    loadMore: () -> Unit,
    isFetchingMoreSheets: Boolean,
    loadMoreError: String?,
    modifier: Modifier = Modifier
) {
    AddSheetPopupContent(
        isTablet = isTablet,
        query = query,
        onQueryChanged = onQueryChanged,
        instrumentQuery = instrumentQuery,
        onInstrumentQueryChanged = onInstrumentQueryChanged,
        onSearchSubmit = onSearchSubmit,
        isLoading = isLoading,
        searchError = searchError,
        results = results,
        selectedResult = selectedResult,
        pdfPath = pdfPath,
        isFetchingPdf = isFetchingPdf,
        pdfFetchError = pdfFetchError,
        isSaving = isSaving,
        onResultClick = onResultClick,
        onClearSelection = onClearSelection,
        onConfirmAndSave = onConfirmAndSave,
        loadMore = loadMore,
        isFetchingMoreSheets = isFetchingMoreSheets,
        loadMoreError = loadMoreError,
        modifier = modifier
    )
}
