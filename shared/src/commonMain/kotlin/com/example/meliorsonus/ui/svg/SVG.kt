package com.example.meliorsonus.ui.svg

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import coil3.ImageLoader
import coil3.compose.AsyncImage
import coil3.compose.LocalPlatformContext
import coil3.request.ImageRequest
import coil3.svg.SvgDecoder
import com.example.meliorsonus.model.svgContent
import com.example.meliorsonus.util.systemFileSystem
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.Json
import okio.Path.Companion.toPath
import okio.buffer
import okio.use
import kotlin.math.ceil

enum class SvgDisplayMode { FULL_SVG, SPLIT, PANEL_ONLY }

/**
 * Renders the SVG sheet music stored as a JSON array or raw XML string at [svgPath].
 */
@Composable
fun SVGView(
    svgPath: String,
    mode: String = "phone",
    currentMeasure: Int = 0,
    pageNextCount: Int = 0,
    pagePreviousCount: Int = 0,
    totalPages: Int = 1,
    displayMode: SvgDisplayMode = SvgDisplayMode.SPLIT,
    onPageChanged: (currentPage: Int) -> Unit = {},
    modifier: Modifier = Modifier,
) {
    when (displayMode) {
        SvgDisplayMode.PANEL_ONLY -> PanelOnlyPlaceholder(modifier = modifier)
        SvgDisplayMode.FULL_SVG,
        SvgDisplayMode.SPLIT -> SvgPager(
            svgPath           = svgPath,
            totalPages        = totalPages,
            displayMode       = displayMode,
            pageNextCount     = pageNextCount,
            pagePreviousCount = pagePreviousCount,
            onPageChanged     = onPageChanged,
            modifier          = modifier,
        )
    }
}

@Composable
private fun PanelOnlyPlaceholder(modifier: Modifier = Modifier) {
    Box(
        modifier = modifier
            .fillMaxSize()
            .background(MaterialTheme.colorScheme.surfaceVariant),
        contentAlignment = Alignment.Center
    ) {
        Text(
            text = "Not implemented",
            style = MaterialTheme.typography.bodyLarge,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
    }
}

@Composable
private fun SvgPager(
    svgPath: String,
    totalPages: Int,
    displayMode: SvgDisplayMode,
    pageNextCount: Int,
    pagePreviousCount: Int,
    onPageChanged: (Int) -> Unit,
    modifier: Modifier = Modifier,
) {
    val context = LocalPlatformContext.current

    // Single shared ImageLoader remembered once for the pager lifecycle
    val sharedImageLoader = remember(context) {
        ImageLoader.Builder(context)
            .components { add(SvgDecoder.Factory()) }
            .build()
    }

    // Load pages asynchronously
    val pages by produceState<List<svgContent>>(initialValue = emptyList(), svgPath) {
        value = withContext(Dispatchers.Default) {
            loadSvgPages(svgPath)
        }
    }

    if (pages.isEmpty()) {
        Box(modifier = modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
            if (svgPath.isNotBlank()) {
                CircularProgressIndicator(color = MaterialTheme.colorScheme.primary)
            }
        }
        return
    }

    val pagerPageCount = when (displayMode) {
        SvgDisplayMode.FULL_SVG -> ceil(pages.size / 2.0).toInt().coerceAtLeast(1)
        else                    -> pages.size.coerceAtLeast(1)
    }

    // Key rememberPagerState on svgPath so switching sheets resets to page 0 cleanly
    val pagerState = key(svgPath) {
        rememberPagerState(pageCount = { pagerPageCount })
    }

    // Handle next/previous page button triggers
    LaunchedEffect(pageNextCount) {
        if (pageNextCount > 0 && pagerState.currentPage < pagerPageCount - 1) {
            pagerState.animateScrollToPage(pagerState.currentPage + 1)
        }
    }
    LaunchedEffect(pagePreviousCount) {
        if (pagePreviousCount > 0 && pagerState.currentPage > 0) {
            pagerState.animateScrollToPage(pagerState.currentPage - 1)
        }
    }

    // Report active SVG page index (1-based) to onPageChanged
    LaunchedEffect(pagerState, displayMode) {
        snapshotFlow { pagerState.currentPage }.collect { pagerPage ->
            val svgPageIndex = when (displayMode) {
                SvgDisplayMode.FULL_SVG -> (pagerPage * 2 + 1).coerceAtMost(pages.size)
                else                    -> (pagerPage + 1).coerceAtMost(pages.size)
            }
            onPageChanged(svgPageIndex)
        }
    }

    HorizontalPager(
        state    = pagerState,
        modifier = modifier.fillMaxSize(),
    ) { pagerPage ->
        when (displayMode) {
            SvgDisplayMode.FULL_SVG -> {
                val firstIdx  = pagerPage * 2
                val secondIdx = firstIdx + 1

                Column(
                    modifier            = Modifier.fillMaxSize(),
                    verticalArrangement = Arrangement.Top,
                ) {
                    SvgPageCell(
                        svgString   = pages.getOrNull(firstIdx)?.svgString,
                        imageLoader = sharedImageLoader,
                        modifier    = Modifier
                            .fillMaxWidth()
                            .weight(1f),
                    )
                    SvgPageCell(
                        svgString   = pages.getOrNull(secondIdx)?.svgString,
                        imageLoader = sharedImageLoader,
                        modifier    = Modifier
                            .fillMaxWidth()
                            .weight(1f),
                    )
                }
            }

            else -> {
                SvgPageCell(
                    svgString   = pages.getOrNull(pagerPage)?.svgString,
                    imageLoader = sharedImageLoader,
                    modifier    = Modifier.fillMaxSize(),
                )
            }
        }
    }
}

@Composable
private fun SvgPageCell(
    svgString: String?,
    imageLoader: ImageLoader,
    modifier: Modifier = Modifier,
) {
    Box(
        modifier         = modifier.background(Color.White),
        contentAlignment = Alignment.Center,
    ) {
        if (svgString.isNullOrBlank()) return@Box

        val context  = LocalPlatformContext.current
        val svgBytes = remember(svgString) { svgString.encodeToByteArray() }

        AsyncImage(
            model = ImageRequest.Builder(context)
                .data(svgBytes)
                .build(),
            imageLoader        = imageLoader,
            contentDescription = "Sheet music page",
            contentScale       = ContentScale.Fit,
            modifier           = Modifier.fillMaxSize(),
        )
    }
}

private fun loadSvgPages(svgPath: String): List<svgContent> {
    if (svgPath.isBlank()) return emptyList()
    return try {
        val raw = systemFileSystem
            .source(svgPath.toPath())
            .buffer()
            .use { it.readUtf8() }

        val trimmed = raw.trim()
        if (trimmed.startsWith("<") || trimmed.startsWith("<?xml")) {
            // Raw SVG fallback
            listOf(svgContent(pageIndex = 1, svgString = raw))
        } else {
            // JSON array of svgContent
            val json = Json { ignoreUnknownKeys = true }
            json.decodeFromString<List<svgContent>>(raw)
                .sortedBy { it.pageIndex }
        }
    } catch (e: Exception) {
        println("SVG.kt: error loading SVG from $svgPath — ${e.message}")
        emptyList()
    }
}
