package com.example.meliorsonus.ui.osmd

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import com.example.meliorsonus.theme.Miscellaneous.GlobalMaterialTheme
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import kotlin.math.roundToInt

@Composable
fun MovableResizableInfoBar(
    title: String,
    details: String,
    modifier: Modifier = Modifier
) {
    var width by remember { mutableStateOf(320.dp) }
    var height by remember { mutableStateOf(180.dp) }
    var offsetX by remember { mutableStateOf(60f) }
    var offsetY by remember { mutableStateOf(120f) }

    Box(
        modifier = modifier
            .offset { IntOffset(offsetX.roundToInt(), offsetY.roundToInt()) }
            .size(width, height)
            .background(GlobalMaterialTheme.colorScheme.surfaceVariant, shape = RoundedCornerShape(12.dp))
            .border(1.dp, GlobalMaterialTheme.colorScheme.outline, shape = RoundedCornerShape(12.dp))
    ) {
        // Drag header to move the window
        Box(
            modifier = Modifier
                .fillMaxWidth()
                .height(40.dp)
                .background(GlobalMaterialTheme.colorScheme.primaryContainer, shape = RoundedCornerShape(topStart = 12.dp, topEnd = 12.dp))
                .pointerInput(Unit) {
                    detectDragGestures { change, dragAmount ->
                        change.consume()
                        offsetX += dragAmount.x
                        offsetY += dragAmount.y
                    }
                }
                .padding(horizontal = 12.dp),
            contentAlignment = Alignment.CenterStart
        ) {
            Text("Practice Controls (Drag)", style = GlobalMaterialTheme.typography.labelSmall, fontWeight = FontWeight.Bold)
        }

        // Details content container
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(top = 48.dp, start = 12.dp, end = 12.dp, bottom = 16.dp)
        ) {
            Text(text = title, style = GlobalMaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold)
            Spacer(Modifier.height(4.dp))
            Text(text = details, style = GlobalMaterialTheme.typography.bodySmall, color = GlobalMaterialTheme.colorScheme.onSurfaceVariant)
        }

        // Drag handle at bottom right corner to resize the bar
        Box(
            modifier = Modifier
                .size(24.dp)
                .align(Alignment.BottomEnd)
                .pointerInput(Unit) {
                    detectDragGestures { change, dragAmount ->
                        change.consume()
                        width = (width + dragAmount.x.toDp()).coerceIn(240.dp, 600.dp)
                        height = (height + dragAmount.y.toDp()).coerceIn(120.dp, 400.dp)
                    }
                }
        ) {
            // Draw resize indicator lines
            androidx.compose.foundation.Canvas(modifier = Modifier.fillMaxSize()) {
                val w = size.width
                val h = size.height
                drawLine(Color.Gray, Offset(w - 6.dp.toPx(), h - 18.dp.toPx()), Offset(w - 18.dp.toPx(), h - 6.dp.toPx()), strokeWidth = 2.dp.toPx())
                drawLine(Color.Gray, Offset(w - 6.dp.toPx(), h - 12.dp.toPx()), Offset(w - 12.dp.toPx(), h - 6.dp.toPx()), strokeWidth = 2.dp.toPx())
            }
        }
    }
}
