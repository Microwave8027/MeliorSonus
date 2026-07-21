package com.example.meliorsonus.ui.home

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier

@Composable
expect fun PdfPreview(
    pdfPath: String,
    modifier: Modifier = Modifier
)
