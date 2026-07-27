package com.example.meliorsonus.ui.home

import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import com.example.meliorsonus.theme.Miscellaneous.GlobalMaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.*
import androidx.compose.runtime.collectAsState
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
import androidx.compose.ui.graphics.Color
import coil3.compose.AsyncImage
import kotlin.math.roundToInt
import io.ktor.util.encodeBase64


/**
 * Stateless add-sheet search popup. Renders one of two inner screens:
 *   1. Search screen — when no result is selected
 *   2. PDF preview screen — when a result has been selected
 *
 * All transitions are driven entirely by [HomeComponent.HomeState] via the parent.
 */
@Composable
fun AddSheetPopupContent(
    isTablet: Boolean,
    query: String,
    onQueryChanged: (String) -> Unit,
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
    onProceed: () -> Unit,
    onClose: () -> Unit,
    showZoomChecker: Boolean,
    zoomLevel: Float,
    isSaving: Boolean = false,
    onZoomLevelChanged: (Float) -> Unit,
    onBackFromZoomChecker: () -> Unit,
    onConfirmZoomAndProceed: () -> Unit,
    verovioComponent: VerovioManagerComponent,
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
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.SpaceBetween
        ) {
            if (showPreview) {
                if (showZoomChecker) {
                    IconButton(onClick = onBackFromZoomChecker) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back to PDF preview")
                    }
                } else {
                    IconButton(onClick = onClearSelection) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back to results")
                    }
                }
                Text(
                    text = if (showZoomChecker) "Configure Zoom" else "Preview Sheet",
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
            IconButton(onClick = onClose) {
                Icon(Icons.Default.Close, contentDescription = "Close")
            }
        }

        Spacer(Modifier.height(12.dp))

        if (!showPreview) {
            SearchScreen(
                query = query,
                onQueryChanged = onQueryChanged,
                onSearchSubmit = onSearchSubmit,
                isLoading = isLoading,
                searchError = searchError,
                results = results,
                onResultClick = onResultClick
            )
        } else if (!showZoomChecker) {
            PdfPreviewScreen(
                result = selectedResult!!,
                pdfPath = pdfPath,
                isFetchingPdf = isFetchingPdf,
                pdfFetchError = pdfFetchError,
                isTablet = isTablet,
                onProceed = onProceed
            )
        } else {
            ZoomCheckerScreen(
                zoomLevel = zoomLevel,
                isSaving = isSaving,
                searchError = searchError,
                onZoomLevelChanged = onZoomLevelChanged,
                onConfirmZoomAndProceed = onConfirmZoomAndProceed,
                verovioComponent = verovioComponent
            )
        }
    }
}

@Composable
private fun ColumnScope.SearchScreen(
    query: String,
    onQueryChanged: (String) -> Unit,
    onSearchSubmit: () -> Unit,
    isLoading: Boolean,
    searchError: String?,
    results: List<SheetSearchResult>,
    onResultClick: (SheetSearchResult) -> Unit
) {
    val keyboardController = LocalSoftwareKeyboardController.current

    OutlinedTextField(
        value = query,
        onValueChange = onQueryChanged,
        placeholder = { Text("Search by title, artist, composer…") },
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
            LazyColumn(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                items(results.take(10)) { result ->
                    SearchResultCard(result = result, onClick = { onResultClick(result) })
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
    onProceed: () -> Unit
) {
    Column(modifier = Modifier.fillMaxWidth()) {
        Text(
            text = result.title, style = GlobalMaterialTheme.typography.titleLarge, color = GlobalMaterialTheme.colorScheme.primary
        )
        Row(modifier = Modifier.fillMaxWidth()){
            Text(
                text = "By", style = GlobalMaterialTheme.typography.bodyMedium, color = GlobalMaterialTheme.colorScheme.onSurface
            )
            if (result.composerName.isNotEmpty() && result.composerName != "Unknown") Text(text = " · ${result.composerName}", style = GlobalMaterialTheme.typography.bodyMedium, color = GlobalMaterialTheme.colorScheme.onSurface)
            if (result.artistName.isNotEmpty() && result.artistName != "Unknown") Text(text = " · ${result.artistName}", style = GlobalMaterialTheme.typography.bodyMedium, color = GlobalMaterialTheme.colorScheme.onSurface)
            if (result.publisher.isNotEmpty() && result.publisher != "Unknown") Text(text = " · ${result.publisher}", style = GlobalMaterialTheme.typography.bodyMedium, color = GlobalMaterialTheme.colorScheme.onSurface)
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

    Spacer(Modifier.height(12.dp))

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

    Spacer(Modifier.height(12.dp))
    Button(onClick = onProceed, modifier = Modifier.fillMaxWidth(), enabled = !isFetchingPdf) {
        Text("Proceed to Practice")
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
                    text = "${result.composerName} • ${result.songLengthBars} bars",
                    style = GlobalMaterialTheme.typography.bodySmall,
                    color = if (isSelected) GlobalMaterialTheme.colorScheme.onPrimaryContainer.copy(alpha = 0.7f) else GlobalMaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis
                )
            }
            Spacer(Modifier.width(8.dp))
            if (result.genres.isNotEmpty()) {
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

@Composable
fun ColumnScope.ZoomCheckerScreen(
    zoomLevel: Float,
    isSaving: Boolean = false,
    searchError: String? = null,
    onZoomLevelChanged: (Float) -> Unit,
    onConfirmZoomAndProceed: () -> Unit,
    verovioComponent: VerovioManagerComponent
) {
    ZoomCheckerContent(
        zoomLevel = zoomLevel,
        isSaving = isSaving,
        searchError = searchError,
        onZoomLevelChanged = onZoomLevelChanged,
        onConfirmZoomAndProceed = onConfirmZoomAndProceed,
        verovioComponent = verovioComponent,
        modifier = Modifier.weight(1f).fillMaxWidth()
    )
}

@Composable
fun ZoomCheckerContent(
    zoomLevel: Float,
    isSaving: Boolean = false,
    searchError: String? = null,
    onZoomLevelChanged: (Float) -> Unit,
    onConfirmZoomAndProceed: () -> Unit,
    verovioComponent: VerovioManagerComponent,
    modifier: Modifier = Modifier
) {
    val verovioState by verovioComponent.state.collectAsState()

    Column(modifier = modifier) {
        Column(
            modifier = Modifier
                .weight(1f)
                .fillMaxWidth()
                .verticalScroll(rememberScrollState())
        ) {
            Column(modifier = Modifier.fillMaxWidth().padding(horizontal = 8.dp)) {
                Text(
                    text = "Edit your zoom",
                    style = GlobalMaterialTheme.typography.titleLarge,
                    color = GlobalMaterialTheme.colorScheme.primary,
                    fontWeight = FontWeight.Bold
                )
                Text(
                    text = "Important: You cannot change your zoom after you proceed",
                    style = GlobalMaterialTheme.typography.bodyMedium,
                    color = GlobalMaterialTheme.colorScheme.onSurfaceVariant
                )
            }

            Spacer(Modifier.height(16.dp))

            // Zoom Slider
            var sliderValue by remember(zoomLevel) { mutableStateOf(zoomLevel) }

            Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.fillMaxWidth()) {
                Text("Zoom: ${(sliderValue * 100).roundToInt()}%", modifier = Modifier.width(100.dp))
                Slider(
                    value = sliderValue,
                    onValueChange = { sliderValue = it },
                    onValueChangeFinished = { onZoomLevelChanged(sliderValue) },
                    valueRange = 0.25f..2.0f,
                    steps = 13, // 13 steps between 0.25 and 2.0 gives roughly 0.125 (12.5%) per step
                    modifier = Modifier.weight(1f)
                )
            }

            Spacer(Modifier.height(16.dp))

            // SVG Preview
            BoxWithConstraints(
                modifier = Modifier
                    .fillMaxWidth()
                    .border(1.dp, GlobalMaterialTheme.colorScheme.outlineVariant, RoundedCornerShape(8.dp))
                    .padding(8.dp),
                contentAlignment = Alignment.Center
            ) {
                val widthPx = constraints.maxWidth.toFloat()

                when {
                    verovioState.isLoading -> {
                        Column(horizontalAlignment = Alignment.CenterHorizontally, modifier = Modifier.padding(vertical = 40.dp)) {
                            CircularProgressIndicator()
                            Spacer(Modifier.height(12.dp))
                            Text("Loading XML...", style = GlobalMaterialTheme.typography.bodyMedium)
                        }
                    }
                    verovioState.error != null -> {
                        Text("Error: ${verovioState.error}", color = GlobalMaterialTheme.colorScheme.error, modifier = Modifier.padding(vertical = 40.dp))
                    }
                    else -> {
                        val density = androidx.compose.ui.platform.LocalDensity.current
                        val widthDp = with(density) { constraints.maxWidth.toDp() }.value.toInt()
                        val heightDp = verovioComponent.fetchScreenHeight(widthDp)
                        val targetHeightPx = with(density) { heightDp.dp.toPx() }

                        val dataUri = remember(zoomLevel, verovioState.currentXmlData, widthPx, targetHeightPx) {
                            val svgString = verovioComponent.renderSvg(zoomLevel, widthPx, targetHeightPx)
                            val base64Svg = svgString.encodeToByteArray().encodeBase64()
                            "data:image/svg+xml;base64,$base64Svg"
                        }
                        Box(
                            modifier = Modifier
                                .fillMaxWidth()
                                .height(heightDp.dp)
                                .clip(RoundedCornerShape(4.dp))
                                .background(Color.White),
                            contentAlignment = Alignment.Center
                        ) {
                            AsyncImage(
                                model = dataUri,
                                contentDescription = "MusicXML SVG Preview",
                                modifier = Modifier.fillMaxSize()
                            )
                        }
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

        Spacer(Modifier.height(16.dp))

        Button(
            onClick = onConfirmZoomAndProceed,
            modifier = Modifier.fillMaxWidth(),
            enabled = verovioState.isLoaded && !isSaving
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
                Text("Proceed")
            }
        }
    }
}
