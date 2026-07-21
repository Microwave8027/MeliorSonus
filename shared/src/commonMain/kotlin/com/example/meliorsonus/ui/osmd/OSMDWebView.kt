package com.example.meliorsonus.ui.osmd

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier

@Composable
expect fun OSMDWebView(
    xmlContent: String,
    mode: String, // "phone" (strip) or "tablet" (paginated)
    zoom: Float,
    currentMeasure: Int,
    pageNextCount: Int = 0,
    pagePreviousCount: Int = 0,
    isDragging: Boolean = false,
    modifier: Modifier = Modifier
)

