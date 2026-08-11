package com.example.meliorsonus.ui.sheetviewer.tabs

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.BarChart
import androidx.compose.material.icons.filled.MusicNote
import androidx.compose.material.icons.filled.Speed
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.example.meliorsonus.model.SheetSearchResult
import com.example.meliorsonus.theme.Miscellaneous.GlobalMaterialTheme
import com.example.meliorsonus.theme.Miscellaneous.LightPurple
import com.example.meliorsonus.theme.Miscellaneous.MildPurple
import org.jetbrains.compose.ui.tooling.preview.Preview

@Composable
fun StatisticsContent(
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
                imageVector = Icons.Default.BarChart,
                contentDescription = null,
                tint = LightPurple,
                modifier = Modifier.size(24.dp)
            )
            Spacer(Modifier.width(8.dp))
            Text(
                text = "Performance Statistics",
                style = GlobalMaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.Bold,
                color = GlobalMaterialTheme.colorScheme.onBackground
            )
        }

        // Summary stat cards
        Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            StatChip(label = "Total Bars", value = sheet.songLengthBars.toString(), modifier = Modifier.weight(1f))
            StatChip(label = "Sessions", value = "12", modifier = Modifier.weight(1f))
            StatChip(label = "Best Score", value = "94%", modifier = Modifier.weight(1f))
        }

        // Practice time card
        StatCard(title = "Total Practice Time") {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Text("This week", color = GlobalMaterialTheme.colorScheme.onSurfaceVariant,
                    style = GlobalMaterialTheme.typography.bodySmall)
                Text("3h 20m", fontWeight = FontWeight.Bold, color = GlobalMaterialTheme.colorScheme.onSurface)
            }
            Spacer(Modifier.height(6.dp))
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Text("All time", color = GlobalMaterialTheme.colorScheme.onSurfaceVariant,
                    style = GlobalMaterialTheme.typography.bodySmall)
                Text("18h 45m", fontWeight = FontWeight.Bold, color = GlobalMaterialTheme.colorScheme.onSurface)
            }
        }

        // Accuracy bar chart (simplified visual)
        StatCard(title = "Accuracy Over Sessions") {
            val bars = listOf(0.72f, 0.78f, 0.80f, 0.75f, 0.87f, 0.90f, 0.94f)
            Row(
                modifier = Modifier.fillMaxWidth().height(80.dp),
                verticalAlignment = Alignment.Bottom,
                horizontalArrangement = Arrangement.spacedBy(6.dp)
            ) {
                bars.forEach { fraction ->
                    Box(
                        modifier = Modifier
                            .weight(1f)
                            .fillMaxHeight(fraction)
                            .clip(RoundedCornerShape(topStart = 4.dp, topEnd = 4.dp))
                            .background(LightPurple.copy(alpha = 0.70f + fraction * 0.30f))
                    )
                }
            }
            Spacer(Modifier.height(4.dp))
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Text("Session 1", style = GlobalMaterialTheme.typography.labelSmall,
                    color = GlobalMaterialTheme.colorScheme.onSurface.copy(alpha = 0.5f))
                Text("Latest", style = GlobalMaterialTheme.typography.labelSmall,
                    color = GlobalMaterialTheme.colorScheme.onSurface.copy(alpha = 0.5f))
            }
        }

        // Instrument info
        if (sheet.instruments.isNotEmpty()) {
            StatCard(title = "Instruments") {
                sheet.instruments.forEach { instrument ->
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Icon(
                            imageVector = Icons.Default.MusicNote,
                            contentDescription = null,
                            tint = MildPurple,
                            modifier = Modifier.size(16.dp)
                        )
                        Spacer(Modifier.width(8.dp))
                        Text(instrument, style = GlobalMaterialTheme.typography.bodySmall,
                            color = GlobalMaterialTheme.colorScheme.onSurface)
                    }
                    Spacer(Modifier.height(4.dp))
                }
            }
        }
    }
}

@Composable
private fun StatChip(label: String, value: String, modifier: Modifier = Modifier) {
    Card(
        colors = CardDefaults.cardColors(containerColor = GlobalMaterialTheme.colorScheme.surface),
        shape = RoundedCornerShape(12.dp),
        modifier = modifier
    ) {
        Column(
            modifier = Modifier.padding(12.dp),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            Text(
                text = value,
                style = GlobalMaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.Bold,
                color = LightPurple
            )
            Text(
                text = label,
                style = GlobalMaterialTheme.typography.labelSmall,
                color = GlobalMaterialTheme.colorScheme.onSurface.copy(alpha = 0.65f)
            )
        }
    }
}

@Composable
private fun StatCard(title: String, content: @Composable ColumnScope.() -> Unit) {
    Card(
        colors = CardDefaults.cardColors(containerColor = GlobalMaterialTheme.colorScheme.surface),
        shape = RoundedCornerShape(12.dp),
        modifier = Modifier.fillMaxWidth()
    ) {
        Column(modifier = Modifier.padding(14.dp)) {
            Text(
                text = title,
                style = GlobalMaterialTheme.typography.bodyMedium,
                fontWeight = FontWeight.SemiBold,
                color = GlobalMaterialTheme.colorScheme.onSurface
            )
            Spacer(Modifier.height(10.dp))
            content()
        }
    }
}

@Preview
@Composable
private fun StatisticsContentPreview() {
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
            StatisticsContent(sheet = sampleSheet)
        }
    }
}
