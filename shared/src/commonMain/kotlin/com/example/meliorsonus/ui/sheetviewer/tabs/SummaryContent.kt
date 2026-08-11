package com.example.meliorsonus.ui.sheetviewer.tabs

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Circle
import androidx.compose.material.icons.automirrored.filled.FormatListBulleted
import androidx.compose.material.icons.filled.MusicNote
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.example.meliorsonus.model.SheetSearchResult
import com.example.meliorsonus.theme.Miscellaneous.GlobalMaterialTheme
import com.example.meliorsonus.theme.Miscellaneous.LightPurple
import com.example.meliorsonus.theme.Miscellaneous.MildPurple
import org.jetbrains.compose.ui.tooling.preview.Preview

@Composable
fun SummaryContent(
    sheet: SheetSearchResult,
    modifier: Modifier = Modifier
) {
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
                imageVector = Icons.AutoMirrored.Filled.FormatListBulleted,
                contentDescription = null,
                tint = LightPurple,
                modifier = Modifier.size(24.dp)
            )
            Spacer(Modifier.width(8.dp))
            Text(
                text = "Sheet Summary",
                style = GlobalMaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.Bold,
                color = GlobalMaterialTheme.colorScheme.onBackground
            )
        }

        // Piece overview card
        Card(
            colors = CardDefaults.cardColors(containerColor = GlobalMaterialTheme.colorScheme.surface),
            shape = RoundedCornerShape(12.dp),
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                SummaryRow(label = "Title", value = sheet.title)
                HorizontalDivider(color = GlobalMaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f))
                SummaryRow(label = "Composer", value = sheet.composer)
                HorizontalDivider(color = GlobalMaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f))
                SummaryRow(label = "Genre", value = sheet.genres.ifBlank { "Unknown" })
                HorizontalDivider(color = GlobalMaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f))
                SummaryRow(label = "Length", value = "${sheet.songLengthBars} bars")
                if (sheet.instruments.isNotEmpty()) {
                    HorizontalDivider(color = GlobalMaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f))
                    SummaryRow(label = "Instruments", value = sheet.instruments.joinToString(", "))
                }
            }
        }

        // Key points bullet list
        Card(
            colors = CardDefaults.cardColors(containerColor = GlobalMaterialTheme.colorScheme.surface),
            shape = RoundedCornerShape(12.dp),
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                Text(
                    text = "Key Points",
                    style = GlobalMaterialTheme.typography.bodyMedium,
                    fontWeight = FontWeight.SemiBold,
                    color = GlobalMaterialTheme.colorScheme.onSurface
                )
                Spacer(Modifier.height(4.dp))
                val bullets = listOf(
                    "Focus on phrasing and musical line throughout.",
                    "Observe all dynamic markings carefully.",
                    "Maintain consistent tempo — use the metronome.",
                    "Memorise the structure: intro, development, recapitulation.",
                    "Pay attention to articulation (slurs vs. staccato)."
                )
                bullets.forEach { bullet ->
                    Row(verticalAlignment = Alignment.Top) {
                        Icon(
                            imageVector = Icons.Default.Circle,
                            contentDescription = null,
                            tint = MildPurple,
                            modifier = Modifier.size(7.dp).padding(top = 5.dp)
                        )
                        Spacer(Modifier.width(8.dp))
                        Text(
                            text = bullet,
                            style = GlobalMaterialTheme.typography.bodySmall,
                            color = GlobalMaterialTheme.colorScheme.onSurface.copy(alpha = 0.80f)
                        )
                    }
                }
            }
        }

        // Practice notes
        Card(
            colors = CardDefaults.cardColors(
                containerColor = LightPurple.copy(alpha = 0.10f)
            ),
            shape = RoundedCornerShape(12.dp),
            modifier = Modifier.fillMaxWidth()
        ) {
            Row(modifier = Modifier.padding(14.dp), verticalAlignment = Alignment.Top) {
                Icon(
                    imageVector = Icons.Default.MusicNote,
                    contentDescription = null,
                    tint = LightPurple,
                    modifier = Modifier.size(18.dp)
                )
                Spacer(Modifier.width(10.dp))
                Column {
                    Text(
                        text = "Practice Note",
                        style = GlobalMaterialTheme.typography.bodySmall,
                        fontWeight = FontWeight.Bold,
                        color = LightPurple
                    )
                    Spacer(Modifier.height(4.dp))
                    Text(
                        text = "This piece benefits greatly from slow, deliberate practice. " +
                            "Begin at 60% tempo and gradually increase as accuracy improves.",
                        style = GlobalMaterialTheme.typography.bodySmall,
                        color = GlobalMaterialTheme.colorScheme.onSurface.copy(alpha = 0.75f)
                    )
                }
            }
        }
    }
}

@Composable
private fun SummaryRow(label: String, value: String) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Text(
            text = label,
            style = GlobalMaterialTheme.typography.bodySmall,
            color = GlobalMaterialTheme.colorScheme.onSurface.copy(alpha = 0.55f),
            modifier = Modifier.weight(0.35f)
        )
        Text(
            text = value,
            style = GlobalMaterialTheme.typography.bodySmall,
            fontWeight = FontWeight.Medium,
            color = GlobalMaterialTheme.colorScheme.onSurface,
            modifier = Modifier.weight(0.65f)
        )
    }
}

@Preview
@Composable
private fun SummaryContentPreview() {
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
            SummaryContent(sheet = sampleSheet)
        }
    }
}
