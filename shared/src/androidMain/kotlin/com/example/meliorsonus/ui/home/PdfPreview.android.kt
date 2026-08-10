package com.example.meliorsonus.ui.home.addSheet

import android.graphics.Bitmap
import android.graphics.pdf.PdfRenderer
import android.os.ParcelFileDescriptor
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import com.example.meliorsonus.theme.Miscellaneous.GlobalMaterialTheme
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.unit.dp
import java.io.File

@Composable
actual fun PdfPreview(
    pdfPath: String,
    modifier: Modifier
) {
    var bitmaps by remember(pdfPath) { mutableStateOf<List<Bitmap>>(emptyList()) }

    LaunchedEffect(pdfPath) {
        if (pdfPath.isEmpty()) return@LaunchedEffect
        val file = File(pdfPath)
        if (file.exists()) {
            try {
                val fileDescriptor = ParcelFileDescriptor.open(file, ParcelFileDescriptor.MODE_READ_ONLY)
                val pdfRenderer = PdfRenderer(fileDescriptor)
                val pageCount = pdfRenderer.pageCount
                val tempBitmaps = mutableListOf<Bitmap>()
                
                // Load first few pages (up to 5) for preview
                val previewCount = minOf(pageCount, 5)
                for (i in 0 until previewCount) {
                    val page = pdfRenderer.openPage(i)
                    // Render page into a bitmap (upscale slightly for clarity)
                    val width = page.width * 2
                    val height = page.height * 2
                    val bitmap = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888)
                    
                    // Pre-fill bitmap with white background to handle transparency
                    val canvas = android.graphics.Canvas(bitmap)
                    canvas.drawColor(android.graphics.Color.WHITE)
                    
                    page.render(bitmap, null, null, PdfRenderer.Page.RENDER_MODE_FOR_DISPLAY)
                    tempBitmaps.add(bitmap)
                    page.close()
                }
                bitmaps = tempBitmaps
                pdfRenderer.close()
                fileDescriptor.close()
            } catch (e: Exception) {
                e.printStackTrace()
            }
        }
    }

    if (bitmaps.isNotEmpty()) {
        LazyColumn(
            modifier = modifier,
            verticalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            items(bitmaps) { bitmap ->
                Card(
                    modifier = Modifier
                        .fillMaxWidth()
                        .aspectRatio(bitmap.width.toFloat() / bitmap.height.toFloat()),
                    shape = RoundedCornerShape(8.dp),
                    colors = CardDefaults.cardColors(containerColor = GlobalMaterialTheme.colorScheme.surface),
                    elevation = CardDefaults.cardElevation(defaultElevation = 2.dp)
                ) {
                    Image(
                        bitmap = bitmap.asImageBitmap(),
                        contentDescription = "PDF Page Preview",
                        modifier = Modifier.fillMaxSize()
                    )
                }
            }
        }
    }
}
