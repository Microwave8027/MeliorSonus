package com.example.meliorsonus.ui.sheetviewer

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Remove
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.unit.dp
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.calculateZoom
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.platform.LocalDensity
import kotlinx.coroutines.delay
import com.arkivanov.decompose.extensions.compose.subscribeAsState
import com.example.meliorsonus.theme.Miscellaneous.GlobalMaterialTheme
import com.example.meliorsonus.ui.osmd.OSMDWebView
import kotlin.math.roundToInt
import androidx.compose.material3.windowsizeclass.ExperimentalMaterial3WindowSizeClassApi
import androidx.compose.material3.windowsizeclass.WindowWidthSizeClass
import androidx.compose.material3.windowsizeclass.WindowSizeClass
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.unit.toSize

/** Controls what the phone layout shows. */
private enum class ViewMode { OSMD_ONLY, SPLIT, PANEL_ONLY }

@OptIn(ExperimentalMaterial3WindowSizeClassApi::class)
@Composable
fun getWindowWidthSize(): WindowWidthSizeClass {
    val density = LocalDensity.current
    val windowInfo = LocalWindowInfo.current
    val size = with(density) { windowInfo.containerSize.toSize().toDpSize() }
    return WindowSizeClass.calculateFromSize(size).widthSizeClass
}

@Composable
fun isTablet(): Boolean = when (getWindowWidthSize()) {
    WindowWidthSizeClass.Medium, WindowWidthSizeClass.Expanded -> true
    else -> false
}
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SheetViewerScreen(
    component: SheetViewerComponent,
    modifier: Modifier = Modifier
) {
    val state by component.state.subscribeAsState()
    val sheet = state.sheet
    var viewMode by remember { mutableStateOf(ViewMode.OSMD_ONLY) }

    LaunchedEffect(Unit) {
        component.fetchMxl()
    }

    BoxWithConstraints(modifier = modifier.fillMaxSize()) {
        val isTablet = isTablet()
        Scaffold(
            modifier = Modifier.fillMaxSize(),
            topBar = {
                SheetTopBar(
                    title = sheet.title,
                    composer = sheet.composerName,
                    onBack = component::onBack,
                    viewMode = if (isTablet) null else viewMode,
                    onViewModeChange = { viewMode = it }
                )
            },
        ) { paddingValues ->
            if (isTablet) {
                Column(modifier = Modifier.fillMaxSize().padding(paddingValues)) {
                    TabletSheetLayout(
                        xmlContent = state.mxl,
                        zoom = state.zoom,
                        currentMeasure = state.currentMeasure,
                        pageNextCount = state.pageNextCount,
                        pagePreviousCount = state.pagePreviousCount,
                        onZoomIn = { component.onZoomIn() },
                        onZoomOut = { component.onZoomOut() },
                        isDragging = state.isDragging,
                    )
                }
            } else {
                var phoneIsDragging by remember { mutableStateOf(false) }

                LaunchedEffect(viewMode) {
                    if (viewMode == ViewMode.SPLIT) {
                        phoneIsDragging = true
                        delay(150) // wait for Android view-system resize to settle
                        phoneIsDragging = false
                    }
                }

                Column(modifier = Modifier.fillMaxSize().padding(paddingValues)) {
                    if (viewMode != ViewMode.PANEL_ONLY) {
                        // Box wrapper lets us stack the loading overlay on top of
                        // the WebView. The existing indicator inside OSMDWebView
                        // renders behind the AndroidView and is never visible.
                        Box(
                            modifier = if (viewMode == ViewMode.SPLIT)
                                Modifier.fillMaxWidth().fillMaxHeight(0.5f)
                            else
                                Modifier.fillMaxSize()
                        ) {
                            PhoneSheetLayout(
                                xmlContent = state.mxl,
                                currentMeasure = state.currentMeasure,
                                zoom = state.zoom,
                                onZoomIn = { component.onZoomIn() },
                                onZoomOut = { component.onZoomOut() },
                                isDragging = state.isDragging || phoneIsDragging,
                                modifier = Modifier.fillMaxSize()
                            )

                            // Overlay: visible while the MusicXML fetch is in progress.
                            // Fades out smoothly once xml arrives.
                            androidx.compose.animation.AnimatedVisibility(
                                visible = state.mxl.isEmpty(),
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
                                            text  = "Loading sheet music\u2026",
                                            style = GlobalMaterialTheme.typography.bodyMedium,
                                            color = MaterialTheme.colorScheme.onSurface
                                                .copy(alpha = 0.7f)
                                        )
                                    }
                                }
                            }
                        }
                    }


                    if (viewMode == ViewMode.SPLIT) {
                        Box(
                            modifier = Modifier.weight(1f).fillMaxWidth(),
                            contentAlignment = Alignment.Center
                        ) {
                            Surface(
                                modifier = Modifier.fillMaxSize(),
                                color = MaterialTheme.colorScheme.surfaceVariant
                            ) {
                                Box(
                                    modifier = Modifier.fillMaxSize(),
                                    contentAlignment = Alignment.Center
                                ) {
                                    Text(
                                        text = "Not implemented",
                                        style = GlobalMaterialTheme.typography.bodyLarge
                                    )
                                }
                            }
                        }
                    }

                    if (viewMode == ViewMode.PANEL_ONLY) {
                        Box(
                            modifier = Modifier.fillMaxSize(),
                            contentAlignment = Alignment.Center
                        ) {
                            Surface(
                                modifier = Modifier.fillMaxSize(),
                                color = MaterialTheme.colorScheme.surfaceVariant
                            ) {
                                Box(
                                    modifier = Modifier.fillMaxSize(),
                                    contentAlignment = Alignment.Center
                                ) {
                                    Text(
                                        text = "Not implemented",
                                        style = GlobalMaterialTheme.typography.bodyLarge
                                    )
                                }
                            }
                        }
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
            if (viewMode != null) {
                // Filled box → OSMD only
                IconButton(onClick = { onViewModeChange(ViewMode.OSMD_ONLY) }) {
                    ViewModeIcon(
                        mode = ViewMode.OSMD_ONLY,
                        selected = viewMode == ViewMode.OSMD_ONLY
                    )
                }
                // Half-filled box → Split
                IconButton(onClick = { onViewModeChange(ViewMode.SPLIT) }) {
                    ViewModeIcon(
                        mode = ViewMode.SPLIT,
                        selected = viewMode == ViewMode.SPLIT
                    )
                }
                // Outlined box → Panel only
                IconButton(onClick = { onViewModeChange(ViewMode.PANEL_ONLY) }) {
                    ViewModeIcon(
                        mode = ViewMode.PANEL_ONLY,
                        selected = viewMode == ViewMode.PANEL_ONLY
                    )
                }
            }
        },
        colors   = TopAppBarDefaults.topAppBarColors(
            containerColor = Color.Black.copy(alpha = 0.35f)
        ),
        modifier = modifier
    )
}

/**
 * Small Canvas-drawn icon representing each [ViewMode].
 *
 * - [ViewMode.OSMD_ONLY]  → fully filled white square
 * - [ViewMode.SPLIT]      → top half filled, bottom half outlined
 * - [ViewMode.PANEL_ONLY] → outlined square only
 *
 * The icon is slightly brighter when [selected].
 */
@Composable
private fun ViewModeIcon(mode: ViewMode, selected: Boolean) {
    val alpha = if (selected) 1f else 0.55f
    val strokeWidth = 2f
    Canvas(modifier = Modifier.size(20.dp)) {
        val s = size.minDimension
        val topLeft = Offset((size.width - s) / 2f, (size.height - s) / 2f)
        val boxSize = Size(s, s)
        when (mode) {
            ViewMode.OSMD_ONLY -> {
                // Fully filled
                drawRect(
                    color = Color.White.copy(alpha = alpha),
                    topLeft = topLeft,
                    size = boxSize
                )
            }
            ViewMode.SPLIT -> {
                // Top half filled
                drawRect(
                    color = Color.White.copy(alpha = alpha),
                    topLeft = topLeft,
                    size = Size(s, s / 2f)
                )
                // Full outline
                drawRect(
                    color = Color.White.copy(alpha = alpha),
                    topLeft = topLeft,
                    size = boxSize,
                    style = androidx.compose.ui.graphics.drawscope.Stroke(width = strokeWidth)
                )
            }
            ViewMode.PANEL_ONLY -> {
                // Outline only
                drawRect(
                    color = Color.White.copy(alpha = alpha),
                    topLeft = topLeft,
                    size = boxSize,
                    style = androidx.compose.ui.graphics.drawscope.Stroke(width = strokeWidth)
                )
            }
        }
    }
}

@Composable
private fun PhoneSheetLayout(
    xmlContent: String,
    currentMeasure: Int,
    zoom: Float,
    onZoomIn: () -> Unit,
    onZoomOut: () -> Unit,
    isDragging: Boolean = false,
    modifier: Modifier = Modifier
) {
    Box(modifier = modifier.fillMaxWidth()) {
        OSMDWebView(
            xmlContent = xmlContent,
            mode = "phone",
            zoom = zoom,
            currentMeasure = currentMeasure,
            isDragging = isDragging
        )

        Row(
            modifier = Modifier
                .align(Alignment.TopEnd)
                .padding(12.dp)
                .background(
                    GlobalMaterialTheme.colorScheme.surface.copy(alpha = 0.92f),
                    shape = RoundedCornerShape(50)
                )
                .border(
                    width = 1.dp,
                    color = GlobalMaterialTheme.colorScheme.outline.copy(alpha = 0.25f),
                    shape = RoundedCornerShape(50)
                )
                .padding(horizontal = 4.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            // Minus
            IconButton(
                onClick = onZoomOut,
                modifier = Modifier.size(36.dp)
            ) {
                Icon(
                    imageVector = Icons.Default.Remove,
                    contentDescription = "Zoom Out",
                    modifier = Modifier.size(16.dp)
                )
            }

            // Centre: "100% 🔍"
            Row(
                verticalAlignment = Alignment.CenterVertically,
                modifier = Modifier.padding(horizontal = 4.dp)
            ) {
                Text(
                    text  = "${(zoom * 100).roundToInt()}%",
                    style = GlobalMaterialTheme.typography.labelSmall
                )
                Spacer(modifier = Modifier.width(3.dp))
                Icon(
                    imageVector = Icons.Default.Search,
                    contentDescription = null,
                    modifier = Modifier.size(14.dp)
                )
            }

            // Plus
            IconButton(
                onClick = onZoomIn,
                modifier = Modifier.size(36.dp)
            ) {
                Icon(
                    imageVector = Icons.Default.Add,
                    contentDescription = "Zoom In",
                    modifier = Modifier.size(16.dp)
                )
            }
        }
    }


}

@Composable
private fun TabletSheetLayout(
    xmlContent: String,
    zoom: Float,
    currentMeasure: Int,
    pageNextCount: Int,
    pagePreviousCount: Int,
    onZoomIn: () -> Unit,
    onZoomOut: () -> Unit,
    isDragging: Boolean = false
) {
    // Keep stable references to callbacks so the gesture coroutine always
    // calls the latest lambdas without restarting on each recomposition.
    val latestOnZoomIn  by rememberUpdatedState(onZoomIn)
    val latestOnZoomOut by rememberUpdatedState(onZoomOut)

    Box(
        modifier = Modifier
            .fillMaxSize()
            .pointerInput(Unit) {
                awaitEachGesture {
                    awaitFirstDown(requireUnconsumed = false)

                    var pinchAccum = 1f

                    do {
                        val event = awaitPointerEvent(PointerEventPass.Initial)

                        if (event.changes.size >= 2) {
                            pinchAccum *= event.calculateZoom()
                            event.changes.forEach { it.consume() }
                            when {
                                pinchAccum >= 1.08f -> {
                                    latestOnZoomIn()
                                    pinchAccum = 1f
                                }
                                pinchAccum <= 0.93f -> {
                                    latestOnZoomOut()
                                    pinchAccum = 1f
                                }
                            }
                        }
                    } while (event.changes.any { it.pressed })
                }
            }
    ) {
        OSMDWebView(
            xmlContent        = xmlContent,
            mode              = "tablet",
            zoom              = zoom,
            currentMeasure    = currentMeasure,
            pageNextCount     = pageNextCount,
            pagePreviousCount = pagePreviousCount,
            modifier          = Modifier.fillMaxSize(),
            isDragging        = isDragging
        )

        // ── Horizontal zoom pill ─ top-right of the WebView area ──────
        Row(
            modifier = Modifier
                .align(Alignment.TopEnd)
                .padding(12.dp)
                .background(
                    GlobalMaterialTheme.colorScheme.surface.copy(alpha = 0.92f),
                    shape = RoundedCornerShape(50)
                )
                .border(
                    width = 1.dp,
                    color = GlobalMaterialTheme.colorScheme.outline.copy(alpha = 0.25f),
                    shape = RoundedCornerShape(50)
                )
                .padding(horizontal = 4.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            // Minus
            IconButton(
                onClick = onZoomOut,
                modifier = Modifier.size(36.dp)
            ) {
                Icon(
                    imageVector = Icons.Default.Remove,
                    contentDescription = "Zoom Out",
                    modifier = Modifier.size(16.dp)
                )
            }

            // Centre: "100% 🔍"
            Row(
                verticalAlignment = Alignment.CenterVertically,
                modifier = Modifier.padding(horizontal = 4.dp)
            ) {
                Text(
                    text  = "${(zoom * 100).roundToInt()}%",
                    style = GlobalMaterialTheme.typography.labelSmall
                )
                Spacer(modifier = Modifier.width(3.dp))
                Icon(
                    imageVector = Icons.Default.Search,
                    contentDescription = null,
                    modifier = Modifier.size(14.dp)
                )
            }

            // Plus
            IconButton(
                onClick = onZoomIn,
                modifier = Modifier.size(36.dp)
            ) {
                Icon(
                    imageVector = Icons.Default.Add,
                    contentDescription = "Zoom In",
                    modifier = Modifier.size(16.dp)
                )
            }
        }
    }
}
