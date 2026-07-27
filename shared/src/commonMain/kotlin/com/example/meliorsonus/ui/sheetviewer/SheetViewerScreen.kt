package com.example.meliorsonus.ui.sheetviewer

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import com.arkivanov.decompose.extensions.compose.subscribeAsState
import com.example.meliorsonus.theme.Miscellaneous.GlobalMaterialTheme
import com.example.meliorsonus.ui.home.ZoomCheckerContent
import com.example.meliorsonus.ui.svg.SVGView
import com.example.meliorsonus.ui.svg.SvgDisplayMode

private enum class ViewMode { FULL_SVG, SPLIT, PANEL_ONLY }

private fun ViewMode.toDisplayMode() = when (this) {
    ViewMode.FULL_SVG   -> SvgDisplayMode.FULL_SVG
    ViewMode.SPLIT      -> SvgDisplayMode.SPLIT
    ViewMode.PANEL_ONLY -> SvgDisplayMode.PANEL_ONLY
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SheetViewerScreen(
    component: SheetViewerComponent,
    modifier: Modifier = Modifier
) {
    val state by component.state.subscribeAsState()
    val sheet = state.sheet
    var viewMode by rememberSaveable { mutableStateOf(ViewMode.SPLIT) }
    var currentPageLabel by rememberSaveable { mutableIntStateOf(1) }

    LaunchedEffect(Unit) {
        component.fetchSVG()
    }

    BoxWithConstraints(modifier = modifier.fillMaxSize()) {
        val isTablet = state.isTablet
        val effectiveDisplayMode = if (isTablet) SvgDisplayMode.FULL_SVG else viewMode.toDisplayMode()

        val pageLabel = if (state.totalPages > 0 && effectiveDisplayMode != SvgDisplayMode.PANEL_ONLY)
            "Page $currentPageLabel / ${state.totalPages}"
        else
            null

        Scaffold(
            modifier = Modifier.fillMaxSize(),
            topBar = {
                SheetTopBar(
                    title            = sheet.title,
                    composer         = sheet.composerName,
                    pageLabel        = pageLabel,
                    onBack           = component::onBack,
                    viewMode         = if (isTablet) null else viewMode,
                    onViewModeChange = { newMode -> viewMode = newMode }
                )
            },
        ) { paddingValues ->
            if (isTablet) {
                Column(modifier = Modifier.fillMaxSize().padding(paddingValues)) {
                    TabletSheetLayout(
                        svgPath           = state.svgPath,
                        totalPages        = state.totalPages,
                        currentMeasure    = state.currentMeasure,
                        pageNextCount     = state.pageNextCount,
                        pagePreviousCount = state.pagePreviousCount,
                        onPageChanged     = { currentPageLabel = it },
                    )
                }
            } else {
                Column(modifier = Modifier.fillMaxSize().padding(paddingValues)) {
                    if (viewMode != ViewMode.PANEL_ONLY) {
                        Box(
                            modifier = if (viewMode == ViewMode.SPLIT)
                                Modifier.fillMaxWidth().fillMaxHeight(0.5f)
                            else
                                Modifier.fillMaxSize()
                        ) {
                            PhoneSheetLayout(
                                svgPath           = state.svgPath,
                                totalPages        = state.totalPages,
                                currentMeasure    = state.currentMeasure,
                                pageNextCount     = state.pageNextCount,
                                pagePreviousCount = state.pagePreviousCount,
                                displayMode       = viewMode.toDisplayMode(),
                                onPageChanged     = { currentPageLabel = it },
                                modifier          = Modifier.fillMaxSize(),
                            )

                            androidx.compose.animation.AnimatedVisibility(
                                visible = state.isLoading,
                                enter = androidx.compose.animation.fadeIn(),
                                exit  = androidx.compose.animation.fadeOut()
                            ) {
                                Box(
                                    modifier = Modifier
                                        .fillMaxSize()
                                        .background(MaterialTheme.colorScheme.surface),
                                    contentAlignment = Alignment.Center
                                ) {
                                    Column(
                                        horizontalAlignment = Alignment.CenterHorizontally,
                                        verticalArrangement = Arrangement.Center
                                    ) {
                                        CircularProgressIndicator(
                                            color = MaterialTheme.colorScheme.primary
                                        )
                                        Spacer(modifier = Modifier.height(12.dp))
                                        Text(
                                            text  = "Loading sheet music…",
                                            style = GlobalMaterialTheme.typography.bodyMedium,
                                            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.7f)
                                        )
                                    }
                                }
                            }
                        }
                    }

                    if (viewMode == ViewMode.SPLIT || viewMode == ViewMode.PANEL_ONLY) {
                        Box(
                            modifier = Modifier
                                .then(if (viewMode == ViewMode.SPLIT) Modifier.weight(1f) else Modifier.fillMaxSize())
                                .fillMaxWidth(),
                            contentAlignment = Alignment.Center
                        ) {
                            Surface(
                                modifier = Modifier.fillMaxSize(),
                                color    = MaterialTheme.colorScheme.surfaceVariant
                            ) {
                                Box(
                                    modifier = Modifier.fillMaxSize(),
                                    contentAlignment = Alignment.Center
                                ) {
                                    Text(
                                        text  = "Not implemented",
                                        style = GlobalMaterialTheme.typography.bodyLarge
                                    )
                                }
                            }
                        }
                    }
                }
            }
        }

        if (state.shouldResize) {
            Dialog(
                onDismissRequest = { component.onCancelResize() },
                properties = DialogProperties(usePlatformDefaultWidth = false)
            ) {
                Surface(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(16.dp),
                    shape          = RoundedCornerShape(16.dp),
                    color          = GlobalMaterialTheme.colorScheme.surface,
                    tonalElevation = 6.dp
                ) {
                    Column(
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(16.dp)
                    ) {
                        Row(
                            modifier          = Modifier.fillMaxWidth(),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            IconButton(onClick = { component.onCancelResize() }) {
                                Icon(
                                    imageVector        = Icons.AutoMirrored.Filled.ArrowBack,
                                    contentDescription = "Cancel"
                                )
                            }
                            Spacer(Modifier.width(8.dp))
                            Text(
                                text       = "Configure Sheet Zoom",
                                style      = GlobalMaterialTheme.typography.titleMedium,
                                fontWeight = FontWeight.Bold
                            )
                        }

                        Spacer(Modifier.height(8.dp))

                        ZoomCheckerContent(
                            zoomLevel               = state.zoomLevel,
                            onZoomLevelChanged      = component::onZoomLevelChanged,
                            onConfirmZoomAndProceed = component::onConfirmResize,
                            verovioComponent        = component.verovioManagerComponent,
                            modifier                = Modifier.weight(1f).fillMaxWidth()
                        )
                    }
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun SheetTopBar(
    title: String,
    composer: String,
    pageLabel: String?,
    onBack: () -> Unit,
    viewMode: ViewMode?,
    onViewModeChange: (ViewMode) -> Unit,
    modifier: Modifier = Modifier
) {
    TopAppBar(
        title = {
            Column {
                Text(
                    text  = title,
                    style = GlobalMaterialTheme.typography.titleMedium,
                    color = Color.White
                )
                Text(
                    text  = composer,
                    style = GlobalMaterialTheme.typography.bodySmall,
                    color = Color.White.copy(alpha = 0.75f)
                )
            }
        },
        navigationIcon = {
            IconButton(onClick = onBack) {
                Icon(
                    imageVector        = Icons.AutoMirrored.Filled.ArrowBack,
                    contentDescription = "Back",
                    tint               = Color.White
                )
            }
        },
        actions = {
            if (pageLabel != null) {
                Text(
                    text     = pageLabel,
                    style    = GlobalMaterialTheme.typography.labelMedium,
                    color    = Color.White,
                    modifier = Modifier.padding(end = 8.dp)
                )
            }

            if (viewMode != null) {
                IconButton(onClick = { onViewModeChange(ViewMode.SPLIT) }) {
                    ViewModeIcon(mode = ViewMode.SPLIT, selected = viewMode == ViewMode.SPLIT)
                }
                IconButton(onClick = { onViewModeChange(ViewMode.PANEL_ONLY) }) {
                    ViewModeIcon(mode = ViewMode.PANEL_ONLY, selected = viewMode == ViewMode.PANEL_ONLY)
                }
                IconButton(onClick = { onViewModeChange(ViewMode.FULL_SVG) }) {
                    ViewModeIcon(mode = ViewMode.FULL_SVG, selected = viewMode == ViewMode.FULL_SVG)
                }
            }
        },
        colors   = TopAppBarDefaults.topAppBarColors(
            containerColor = Color.Black
        ),
        modifier = modifier
    )
}

@Composable
private fun ViewModeIcon(mode: ViewMode, selected: Boolean) {
    val alpha       = if (selected) 1f else 0.55f
    val strokeWidth = 2f
    Canvas(modifier = Modifier.size(20.dp)) {
        val s       = size.minDimension
        val topLeft = Offset((size.width - s) / 2f, (size.height - s) / 2f)
        val boxSize = Size(s, s)
        when (mode) {
            ViewMode.FULL_SVG -> {
                drawRect(
                    color   = Color.White.copy(alpha = alpha),
                    topLeft = topLeft,
                    size    = boxSize
                )
            }
            ViewMode.SPLIT -> {
                drawRect(
                    color   = Color.White.copy(alpha = alpha),
                    topLeft = topLeft,
                    size    = Size(s, s / 2f)
                )
                drawRect(
                    color   = Color.White.copy(alpha = alpha),
                    topLeft = topLeft,
                    size    = boxSize,
                    style   = androidx.compose.ui.graphics.drawscope.Stroke(width = strokeWidth)
                )
            }
            ViewMode.PANEL_ONLY -> {
                drawRect(
                    color   = Color.White.copy(alpha = alpha),
                    topLeft = topLeft,
                    size    = boxSize,
                    style   = androidx.compose.ui.graphics.drawscope.Stroke(width = strokeWidth)
                )
            }
        }
    }
}

@Composable
private fun PhoneSheetLayout(
    svgPath: String,
    totalPages: Int,
    currentMeasure: Int,
    pageNextCount: Int,
    pagePreviousCount: Int,
    displayMode: SvgDisplayMode,
    onPageChanged: (Int) -> Unit,
    modifier: Modifier = Modifier,
) {
    Box(modifier = modifier.fillMaxWidth()) {
        SVGView(
            svgPath           = svgPath,
            mode              = "phone",
            currentMeasure    = currentMeasure,
            pageNextCount     = pageNextCount,
            pagePreviousCount = pagePreviousCount,
            totalPages        = totalPages,
            displayMode       = displayMode,
            onPageChanged     = onPageChanged,
        )
    }
}

@Composable
private fun TabletSheetLayout(
    svgPath: String,
    totalPages: Int,
    currentMeasure: Int,
    pageNextCount: Int,
    pagePreviousCount: Int,
    onPageChanged: (Int) -> Unit,
) {
    Box(modifier = Modifier.fillMaxSize()) {
        SVGView(
            svgPath           = svgPath,
            mode              = "tablet",
            currentMeasure    = currentMeasure,
            pageNextCount     = pageNextCount,
            pagePreviousCount = pagePreviousCount,
            totalPages        = totalPages,
            displayMode       = SvgDisplayMode.FULL_SVG,
            onPageChanged     = onPageChanged,
            modifier          = Modifier.fillMaxSize(),
        )
    }
}