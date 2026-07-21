package com.example.meliorsonus.ui.home

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.draggable
import androidx.compose.foundation.gestures.rememberDraggableState
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import kotlin.math.roundToInt

@Composable
fun SwipeToDeleteContainer(
    onDelete: () -> Unit,
    modifier: Modifier = Modifier,
    content: @Composable () -> Unit
) {
    BoxWithConstraints(modifier = modifier.fillMaxWidth()) {
        val width = maxWidth
        val widthPx = with(LocalDensity.current) { width.toPx() }
        val maxDragDistance = -widthPx / 3f // Restrict to exactly 1/3 of the device width

        var offsetX by remember { mutableStateOf(0f) }
        val draggableState = rememberDraggableState { delta ->
            offsetX = (offsetX + delta).coerceIn(maxDragDistance, 0f)
        }

        Box(modifier = Modifier.fillMaxWidth().height(IntrinsicSize.Min)) {
            // Background Delete Button (1/3 width, Red, Trash Can)
            Box(
                modifier = Modifier
                    .align(Alignment.CenterEnd)
                    .width(width / 3)
                    .fillMaxHeight()
                    .padding(vertical = 2.dp) // align visually with card outline/margins
                    .clip(RoundedCornerShape(8.dp))
                    .background(Color.Red)
                    .clickable { onDelete() },
                contentAlignment = Alignment.Center
            ) {
                Icon(
                    imageVector = Icons.Default.Delete,
                    contentDescription = "Delete",
                    tint = Color.White
                )
            }

            // Foreground Content (slides left up to 1/3 width)
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .offset { IntOffset(offsetX.roundToInt(), 0) }
                    .draggable(
                        state = draggableState,
                        orientation = Orientation.Horizontal,
                        onDragStopped = {
                            offsetX = if (offsetX < maxDragDistance / 2) {
                                maxDragDistance
                            } else {
                                0f
                            }
                        }
                    )
            ) {
                content()
            }
        }
    }
}
