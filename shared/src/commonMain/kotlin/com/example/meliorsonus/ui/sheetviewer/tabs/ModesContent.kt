package com.example.meliorsonus.ui.sheetviewer.tabs

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.CallSplit
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.example.meliorsonus.model.SheetSearchResult
import com.example.meliorsonus.theme.Miscellaneous.GlobalMaterialTheme
import com.example.meliorsonus.theme.Miscellaneous.LightPurple
import org.jetbrains.compose.ui.tooling.preview.Preview

enum class PracticeMode(
    val label: String,
    val description: String,
    val icon: ImageVector
) {
    FULL_PLAYBACK(
        label = "Full Playback",
        description = "Play the piece from start to finish. Ideal for performance run-throughs.",
        icon = Icons.Default.PlayArrow
    ),
    LOOP_SECTION(
        label = "Loop Section",
        description = "Repeat a selected range of measures indefinitely until mastered.",
        icon = Icons.Default.Repeat
    ),
    SLOW_PRACTICE(
        label = "Slow Practice",
        description = "Play at a reduced tempo to build accuracy before increasing speed.",
        icon = Icons.Default.SlowMotionVideo
    ),
    HANDS_SEPARATE(
        label = "Hands Separate",
        description = "Practice individual parts independently for polyphonic instruments.",
        icon = Icons.AutoMirrored.Filled.CallSplit
    ),
    SIGHT_READING(
        label = "Sight Reading",
        description = "Auto-advance the score in real time. No pausing or looping allowed.",
        icon = Icons.Default.RemoveRedEye
    )
}

@Composable
fun ModesContent(
    sheet: SheetSearchResult,
    modifier: Modifier = Modifier
) {
    var selectedMode by remember { mutableStateOf(PracticeMode.FULL_PLAYBACK) }

    Column(
        modifier = modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        // Header
        Row(verticalAlignment = Alignment.CenterVertically) {
            Icon(
                imageVector = Icons.Default.Tune,
                contentDescription = null,
                tint = LightPurple,
                modifier = Modifier.size(24.dp)
            )
            Spacer(Modifier.width(8.dp))
            Text(
                text = "Practice Modes",
                style = GlobalMaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.Bold,
                color = GlobalMaterialTheme.colorScheme.onBackground
            )
        }

        Text(
            text = "Select how you want to practice \"${sheet.title}\".",
            style = GlobalMaterialTheme.typography.bodySmall,
            color = GlobalMaterialTheme.colorScheme.onSurface.copy(alpha = 0.65f)
        )

        PracticeMode.entries.forEach { mode ->
            ModeCard(
                mode = mode,
                isSelected = selectedMode == mode,
                onSelect = { selectedMode = mode }
            )
        }

        Spacer(Modifier.height(8.dp))
        Button(
            onClick = { /* Start selected mode */ },
            modifier = Modifier.fillMaxWidth(),
            colors = ButtonDefaults.buttonColors(containerColor = LightPurple)
        ) {
            Icon(imageVector = selectedMode.icon, contentDescription = null)
            Spacer(Modifier.width(8.dp))
            Text(
                text = "Start ${selectedMode.label}",
                fontWeight = FontWeight.SemiBold
            )
        }
    }
}

@Composable
private fun ModeCard(
    mode: PracticeMode,
    isSelected: Boolean,
    onSelect: () -> Unit
) {
    val borderColor = if (isSelected) LightPurple else GlobalMaterialTheme.colorScheme.surface
    Card(
        onClick = onSelect,
        colors = CardDefaults.cardColors(
            containerColor = if (isSelected)
                LightPurple.copy(alpha = 0.10f)
            else
                GlobalMaterialTheme.colorScheme.surface
        ),
        shape = RoundedCornerShape(12.dp),
        modifier = Modifier.fillMaxWidth(),
        border = if (isSelected) CardDefaults.outlinedCardBorder() else null
    ) {
        Row(
            modifier = Modifier.padding(14.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Surface(
                color = if (isSelected) LightPurple.copy(alpha = 0.20f)
                else GlobalMaterialTheme.colorScheme.onSurface.copy(alpha = 0.06f),
                shape = RoundedCornerShape(10.dp)
            ) {
                Icon(
                    imageVector = mode.icon,
                    contentDescription = null,
                    tint = if (isSelected) LightPurple
                    else GlobalMaterialTheme.colorScheme.onSurface.copy(alpha = 0.60f),
                    modifier = Modifier.padding(8.dp).size(22.dp)
                )
            }
            Spacer(Modifier.width(12.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = mode.label,
                    style = GlobalMaterialTheme.typography.bodyMedium,
                    fontWeight = FontWeight.SemiBold,
                    color = if (isSelected) LightPurple
                    else GlobalMaterialTheme.colorScheme.onSurface
                )
                Text(
                    text = mode.description,
                    style = GlobalMaterialTheme.typography.bodySmall,
                    color = GlobalMaterialTheme.colorScheme.onSurface.copy(alpha = 0.65f)
                )
            }
            if (isSelected) {
                Icon(
                    imageVector = Icons.Default.CheckCircle,
                    contentDescription = "Selected",
                    tint = LightPurple,
                    modifier = Modifier.size(20.dp)
                )
            }
        }
    }
}

@Preview
@Composable
private fun ModesContentPreview() {
    val sampleSheet = SheetSearchResult(
        mxl = "sample.mxl",
        pdf = "sample.pdf",
        title = "Moonlight Sonata",
        composer = "Ludwig van Beethoven",
        songLengthBars = 64,
        genres = "Classical",
        instruments = listOf("Piano")
    )
    GlobalMaterialTheme {
        Surface {
            ModesContent(sheet = sampleSheet)
        }
    }
}
