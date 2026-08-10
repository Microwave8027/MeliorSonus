package com.example.meliorsonus.ui.home.addSheet

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import com.example.meliorsonus.theme.Miscellaneous.GlobalMaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.example.meliorsonus.model.SheetSearchResult
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.draw.clip
import com.example.meliorsonus.ui.home.tabDirectories.MusicSheetThumbnail

/**
 * Stateless add-sheet search popup. Renders one of two inner screens:
 *   1. Search screen — when no result is selected
 *   2. PDF preview screen — when a result has been selected
 *
 * All transitions are driven entirely by [com.example.meliorsonus.ui.home.core.HomeComponent.HomeState] via the parent.
 */
@Composable
fun AddSheetPopupContent(
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
    onResultClick: (SheetSearchResult) -> Unit,
    onClearSelection: () -> Unit,
    onConfirmAndSave: () -> Unit,
    isSaving: Boolean = false,
    loadMore: () -> Unit,
    isFetchingMoreSheets: Boolean,
    loadMoreError: String?,
    modifier: Modifier = Modifier
) {
    val showPreview = selectedResult != null

    Column(
        modifier = modifier
            .fillMaxSize()
            .padding(16.dp)
    ) {
        // Header
        Row(
            modifier = Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically
        ) {
            if (showPreview) {
                IconButton(onClick = onClearSelection) {
                    Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back to results")
                }
                Text(
                    text = "Preview Sheet",
                    style = GlobalMaterialTheme.typography.titleLarge,
                    fontWeight = FontWeight.Bold,
                    modifier = Modifier.weight(1f)
                )
            } else {
                Text(
                    text = "Find Sheet Music",
                    style = GlobalMaterialTheme.typography.titleLarge,
                    fontWeight = FontWeight.Bold,
                    modifier = Modifier.weight(1f)
                )
            }
        }

        Spacer(Modifier.height(8.dp))

        if (!showPreview) {
            SearchScreen(
                query = query,
                onQueryChanged = onQueryChanged,
                instrumentQuery = instrumentQuery,
                onInstrumentQueryChanged = onInstrumentQueryChanged,
                onSearchSubmit = onSearchSubmit,
                isLoading = isLoading,
                searchError = searchError,
                results = results,
                onResultClick = onResultClick,
                loadMore = loadMore,
                isFetchingMoreSheets = isFetchingMoreSheets,
                loadMoreError = loadMoreError
            )
        } else {
            PdfPreviewScreen(
                result = selectedResult!!,
                pdfPath = pdfPath,
                isFetchingPdf = isFetchingPdf,
                pdfFetchError = pdfFetchError,
                isTablet = isTablet,
                isSaving = isSaving,
                searchError = searchError,
                onConfirmAndSave = onConfirmAndSave
            )
        }
    }
}

@Composable
private fun ColumnScope.SearchScreen(
    query: String,
    onQueryChanged: (String) -> Unit,
    instrumentQuery: String,
    onInstrumentQueryChanged: (String) -> Unit,
    onSearchSubmit: () -> Unit,
    isLoading: Boolean,
    isFetchingMoreSheets: Boolean,
    searchError: String?,
    loadMoreError: String?,
    results: List<SheetSearchResult>,
    onResultClick: (SheetSearchResult) -> Unit,
    loadMore: () -> Unit
) {
    val keyboardController = LocalSoftwareKeyboardController.current
    val listState = rememberLazyListState()

    val shouldLoadMore = remember {
        derivedStateOf {
            val totalItems = listState.layoutInfo.totalItemsCount
            val lastVisibleItemIndex = listState.layoutInfo.visibleItemsInfo.lastOrNull()?.index ?: 0
            totalItems > 0 && lastVisibleItemIndex >= totalItems - 3
        }
    }

    LaunchedEffect(shouldLoadMore.value, isFetchingMoreSheets, loadMoreError) {
        if (shouldLoadMore.value && 
            !isLoading && 
            !isFetchingMoreSheets && 
            searchError == null && 
            loadMoreError == null
        ) {
            loadMore()
        }
    }

    Column(modifier = Modifier.fillMaxWidth()) {
        OutlinedTextField(
            value = query,
            onValueChange = onQueryChanged,
            placeholder = { Text("Search by title..") },
            leadingIcon = { Icon(Icons.Default.Search, contentDescription = "Search") },
            trailingIcon = {
                if (isLoading) {
                    CircularProgressIndicator(modifier = Modifier.size(20.dp), strokeWidth = 2.dp)
                } else if (query.isNotEmpty()) {
                    IconButton(onClick = { onQueryChanged("") }) {
                        Icon(Icons.Default.Clear, contentDescription = "Clear")
                    }
                }
            },
            singleLine = true,
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(8.dp),
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
            keyboardActions = KeyboardActions(onSearch = {
                onSearchSubmit()
                keyboardController?.hide()
            })
        )

        Spacer(Modifier.height(8.dp))

        OutlinedTextField(
            value = instrumentQuery,
            onValueChange = onInstrumentQueryChanged,
            placeholder = { Text("Add instrument..") },
            leadingIcon = { Icon(Icons.Default.MusicNote, contentDescription = "Instrument") },
            trailingIcon = {
                if (instrumentQuery.isNotEmpty()) {
                    IconButton(onClick = { onInstrumentQueryChanged("") }) {
                        Icon(Icons.Default.Clear, contentDescription = "Clear")
                    }
                }
            },
            singleLine = true,
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(8.dp),
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
            keyboardActions = KeyboardActions(onSearch = {
                onSearchSubmit()
                keyboardController?.hide()
            })
        )
    }

    Spacer(Modifier.height(12.dp))

    when {
        isLoading -> {
            Box(modifier = Modifier.weight(1f).fillMaxWidth(), contentAlignment = Alignment.Center) {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    CircularProgressIndicator()
                    Spacer(Modifier.height(12.dp))
                    Text("Searching…", style = GlobalMaterialTheme.typography.bodyMedium, color = GlobalMaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
        }
        searchError != null -> {
            Box(modifier = Modifier.weight(1f).fillMaxWidth(), contentAlignment = Alignment.Center) {
                Column(horizontalAlignment = Alignment.CenterHorizontally, modifier = Modifier.padding(24.dp)) {
                    Icon(Icons.Default.Warning, contentDescription = "Error", modifier = Modifier.size(40.dp), tint = GlobalMaterialTheme.colorScheme.error)
                    Spacer(Modifier.height(12.dp))
                    Text(searchError, style = GlobalMaterialTheme.typography.bodyMedium, color = GlobalMaterialTheme.colorScheme.error, textAlign = TextAlign.Center)
                    Spacer(Modifier.height(16.dp))
                    OutlinedButton(onClick = onSearchSubmit) { Text("Retry") }
                }
            }
        }
        results.isNotEmpty() -> {
            LazyColumn(state = listState, modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                items(results) { result ->
                    SearchResultCard(result = result, onClick = { onResultClick(result) })
                }

                if (isFetchingMoreSheets){
                    item {
                        Box(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(16.dp),
                            contentAlignment = Alignment.Center
                        ) {
                            CircularProgressIndicator()
                        }
                    }
                }
                if (!loadMoreError.isNullOrBlank()){
                    item {
                        Box(modifier = Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
                            Column(horizontalAlignment = Alignment.CenterHorizontally, modifier = Modifier.padding(24.dp)) {
                                Icon(Icons.Default.Warning, contentDescription = "Error", modifier = Modifier.size(40.dp), tint = GlobalMaterialTheme.colorScheme.error)
                                Spacer(Modifier.height(12.dp))
                                Text(loadMoreError, style = GlobalMaterialTheme.typography.bodyMedium, color = GlobalMaterialTheme.colorScheme.error, textAlign = TextAlign.Center)
                                Spacer(Modifier.height(16.dp))
                                OutlinedButton(onClick = loadMore) { Text("Retry") }
                            }
                        }
                    }
                }
            }
        }
        query.isNotEmpty() -> {
            Box(modifier = Modifier.weight(1f).fillMaxWidth(), contentAlignment = Alignment.Center) {
                Text("No results found. Try a different search.", style = GlobalMaterialTheme.typography.bodyMedium, color = GlobalMaterialTheme.colorScheme.onSurfaceVariant, textAlign = TextAlign.Center)
            }
        }
        else -> {
            Box(modifier = Modifier.weight(1f).fillMaxWidth(), contentAlignment = Alignment.Center) {
                Text("Search for sheet music to get started.", style = GlobalMaterialTheme.typography.bodyMedium, color = GlobalMaterialTheme.colorScheme.onSurfaceVariant, textAlign = TextAlign.Center)
            }
        }
    }
}

@Composable
private fun ColumnScope.PdfPreviewScreen(
    result: SheetSearchResult,
    pdfPath: String?,
    isFetchingPdf: Boolean,
    pdfFetchError: String?,
    isTablet: Boolean,
    isSaving: Boolean,
    searchError: String?,
    onConfirmAndSave: () -> Unit
) {
    Column(modifier = Modifier.fillMaxWidth().padding(top = 4.dp)) {
        Text(
            text = result.title, style = GlobalMaterialTheme.typography.titleLarge, color = GlobalMaterialTheme.colorScheme.primary
        )
        Row(modifier = Modifier.fillMaxWidth()){
            Text(
                text = "By", style = GlobalMaterialTheme.typography.bodyMedium, color = GlobalMaterialTheme.colorScheme.onSurface
            )
            if (result.composer.isNotEmpty() && result.composer != "Unknown") Text(text = " · ${result.composer}", style = GlobalMaterialTheme.typography.bodyMedium, color = GlobalMaterialTheme.colorScheme.onSurface)
        }
        if (result.genres.isNotEmpty()) Text(
            text = "Genres: ${result.genres}, ",
            style = GlobalMaterialTheme.typography.bodySmall,
            color = GlobalMaterialTheme.colorScheme.onSurfaceVariant
        )
        if (result.instruments.isNotEmpty()) Text(
            text = "Instruments: ${
                result.instruments.joinToString(
                    ", "
                )
            }",
            style = GlobalMaterialTheme.typography.bodySmall,
            color = GlobalMaterialTheme.colorScheme.onSurfaceVariant
        )
    }

    Spacer(Modifier.height(8.dp))

    Box(
        modifier = Modifier
            .weight(1f)
            .fillMaxWidth()
            .background(GlobalMaterialTheme.colorScheme.surfaceVariant, shape = RoundedCornerShape(12.dp)),
        contentAlignment = Alignment.Center
    ) {
        when {
            isFetchingPdf -> {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    CircularProgressIndicator()
                    Spacer(Modifier.height(12.dp))
                    Text("Loading PDF preview…", style = GlobalMaterialTheme.typography.bodyMedium, color = GlobalMaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
            pdfFetchError != null -> {
                Column(horizontalAlignment = Alignment.CenterHorizontally, modifier = Modifier.padding(24.dp)) {
                    Icon(Icons.Default.Warning, contentDescription = "Error", modifier = Modifier.size(36.dp), tint = GlobalMaterialTheme.colorScheme.error)
                    Spacer(Modifier.height(8.dp))
                    Text(pdfFetchError, style = GlobalMaterialTheme.typography.bodyMedium, color = GlobalMaterialTheme.colorScheme.error, textAlign = TextAlign.Center)
                }
            }
            pdfPath != null -> {
                PdfPreview(
                    pdfPath = pdfPath,
                    modifier = Modifier.fillMaxSize()
                )
            }
            else -> {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Icon(Icons.Default.Info, contentDescription = "PDF", modifier = Modifier.size(40.dp), tint = GlobalMaterialTheme.colorScheme.onSurfaceVariant)
                    Spacer(Modifier.height(8.dp))
                    Text("PDF will appear here", style = GlobalMaterialTheme.typography.bodyMedium, color = GlobalMaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
        }
    }

    if (searchError != null) {
        Spacer(Modifier.height(8.dp))
        Text(
            text = searchError,
            color = GlobalMaterialTheme.colorScheme.error,
            style = GlobalMaterialTheme.typography.bodySmall,
            textAlign = TextAlign.Center,
            modifier = Modifier.fillMaxWidth()
        )
    }

    Spacer(Modifier.height(12.dp))
    Button(
        onClick = onConfirmAndSave,
        modifier = Modifier.fillMaxWidth(),
        enabled = !isFetchingPdf && !isSaving
    ) {
        if (isSaving) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                CircularProgressIndicator(
                    modifier = Modifier.size(18.dp),
                    strokeWidth = 2.dp,
                    color = MaterialTheme.colorScheme.onPrimary
                )
                Spacer(Modifier.width(8.dp))
                Text("Saving...")
            }
        } else {
            Text("Proceed to Practice")
        }
    }
}

@Composable
fun SearchResultCard(
    result: SheetSearchResult,
    onClick: () -> Unit,
    isSelected: Boolean = false,
    modifier: Modifier = Modifier
) {
    Card(
        modifier = modifier.fillMaxWidth(),
        onClick = onClick,
        shape = RoundedCornerShape(8.dp),
        colors = CardDefaults.cardColors(
            containerColor = if (isSelected) GlobalMaterialTheme.colorScheme.primaryContainer
            else GlobalMaterialTheme.colorScheme.surface
        ),
        elevation = CardDefaults.cardElevation(defaultElevation = if (isSelected) 4.dp else 1.dp)
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(12.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            MusicSheetThumbnail(
                modifier = Modifier
                    .size(50.dp)
                    .clip(RoundedCornerShape(6.dp))
                    .background(GlobalMaterialTheme.colorScheme.background)
            )
            Spacer(Modifier.width(16.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = result.title,
                    style = GlobalMaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.SemiBold,
                    color = if (isSelected) GlobalMaterialTheme.colorScheme.onPrimaryContainer else GlobalMaterialTheme.colorScheme.onSurface,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis
                )
                Text(
                    text = "${result.composer} • ${result.songLengthBars} bars",
                    style = GlobalMaterialTheme.typography.bodySmall,
                    color = if (isSelected) GlobalMaterialTheme.colorScheme.onPrimaryContainer.copy(alpha = 0.7f) else GlobalMaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis
                )
            }
            Spacer(Modifier.width(8.dp))
            if (result.genres.isNotEmpty() && result.genres != "Unknown") {
                Surface(
                    color = if (isSelected) GlobalMaterialTheme.colorScheme.primary else GlobalMaterialTheme.colorScheme.primaryContainer,
                    shape = RoundedCornerShape(4.dp)
                ) {
                    Text(
                        text = result.genres,
                        style = GlobalMaterialTheme.typography.bodySmall,
                        fontWeight = FontWeight.Medium,
                        color = if (isSelected) GlobalMaterialTheme.colorScheme.onPrimary else GlobalMaterialTheme.colorScheme.onPrimaryContainer,
                        modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp)
                    )
                }
            }
        }
    }
}
