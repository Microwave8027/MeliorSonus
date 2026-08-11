package com.example.meliorsonus.ui.sheetviewer.tabs

import androidx.compose.animation.core.*
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Pause
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Remove
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Timer
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.example.meliorsonus.model.SheetSearchResult
import com.example.meliorsonus.theme.Miscellaneous.GlobalMaterialTheme
import com.example.meliorsonus.theme.Miscellaneous.LightPurple
import com.example.meliorsonus.theme.Miscellaneous.MildPurple
import org.jetbrains.compose.ui.tooling.preview.Preview

@Composable
fun MetronomeContent(
    sheet: SheetSearchResult,
    modifier: Modifier = Modifier
) {
    var bpm by remember { mutableStateOf(120) }
    var isPlaying by remember { mutableStateOf(false) }

    // Pendulum animation
    val infiniteTransition = rememberInfiniteTransition(label = "metronome")
    val pendulumAngle by infiniteTransition.animateFloat(
        initialValue = -30f,
        targetValue = 30f,
        animationSpec = infiniteRepeatable(
            animation = tween(
                durationMillis = (60_000 / bpm),
                easing = EaseInOut
            ),
            repeatMode = RepeatMode.Reverse
        ),
        label = "pendulum"
    )

    Column(
        modifier = modifier
            .fillMaxSize()
            .padding(horizontal = 16.dp, vertical = 12.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(20.dp)
    ) {
        Card(
            colors = CardDefaults.cardColors(containerColor = GlobalMaterialTheme.colorScheme.surface),
            shape = RoundedCornerShape(16.dp),
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(
                modifier = Modifier.padding(20.dp),
                horizontalAlignment = Alignment.CenterHorizontally
            ) {
                Text(
                    text = "$bpm",
                    style = GlobalMaterialTheme.typography.headlineLarge.copy(fontSize = 56.sp),
                    fontWeight = FontWeight.ExtraBold,
                    color = LightPurple
                )
                Text(
                    text = "BPM",
                    style = GlobalMaterialTheme.typography.bodyMedium,
                    color = GlobalMaterialTheme.colorScheme.onSurface.copy(alpha = 0.55f)
                )
                Spacer(Modifier.height(8.dp))
                Text(
                    text = tempoLabel(bpm),
                    style = GlobalMaterialTheme.typography.bodySmall,
                    fontWeight = FontWeight.Medium,
                    color = MildPurple
                )
            }
        }

        // Pendulum visual
        if (isPlaying) {
            Box(
                modifier = Modifier
                    .size(width = 8.dp, height = 120.dp)
                    .rotate(pendulumAngle)
                    .clip(RoundedCornerShape(4.dp))
                    .background(LightPurple)
            )
        } else {
            Box(
                modifier = Modifier
                    .size(width = 8.dp, height = 120.dp)
                    .clip(RoundedCornerShape(4.dp))
                    .background(GlobalMaterialTheme.colorScheme.onSurface.copy(alpha = 0.15f))
            )
        }

        // BPM controls
        Row(
            horizontalArrangement = Arrangement.spacedBy(16.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            // -10
            OutlinedIconButton(
                onClick = { if (bpm > 20) bpm -= 10 },
                modifier = Modifier.size(48.dp)
            ) {
                Text("-10", fontSize = 10.sp, fontWeight = FontWeight.Bold,
                    color = GlobalMaterialTheme.colorScheme.onSurface)
            }
            // -1
            FilledIconButton(
                onClick = { if (bpm > 20) bpm -= 1 },
                colors = IconButtonDefaults.filledIconButtonColors(
                    containerColor = LightPurple.copy(alpha = 0.20f)
                ),
                modifier = Modifier.size(48.dp)
            ) {
                Icon(Icons.Default.Remove, contentDescription = "Decrease", tint = LightPurple)
            }
            // Play/Pause
            FilledIconButton(
                onClick = { isPlaying = !isPlaying },
                colors = IconButtonDefaults.filledIconButtonColors(containerColor = LightPurple),
                modifier = Modifier.size(64.dp)
            ) {
                Icon(
                    imageVector = if (isPlaying) Icons.Default.Pause else Icons.Default.PlayArrow,
                    contentDescription = if (isPlaying) "Pause" else "Play",
                    tint = androidx.compose.ui.graphics.Color.White,
                    modifier = Modifier.size(32.dp)
                )
            }
            // +1
            FilledIconButton(
                onClick = { if (bpm < 300) bpm += 1 },
                colors = IconButtonDefaults.filledIconButtonColors(
                    containerColor = LightPurple.copy(alpha = 0.20f)
                ),
                modifier = Modifier.size(48.dp)
            ) {
                Icon(Icons.Default.Add, contentDescription = "Increase", tint = LightPurple)
            }
            // +10
            OutlinedIconButton(
                onClick = { if (bpm < 300) bpm += 10 },
                modifier = Modifier.size(48.dp)
            ) {
                Text("+10", fontSize = 10.sp, fontWeight = FontWeight.Bold,
                    color = GlobalMaterialTheme.colorScheme.onSurface)
            }
        }

        // BPM slider
        Slider(
            value = bpm.toFloat(),
            onValueChange = { bpm = it.toInt() },
            valueRange = 20f..300f,
            steps = 0,
            colors = SliderDefaults.colors(
                thumbColor = LightPurple,
                activeTrackColor = LightPurple,
                inactiveTrackColor = LightPurple.copy(alpha = 0.25f)
            ),
            modifier = Modifier.fillMaxWidth()
        )
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween
        ) {
            Text("20", style = GlobalMaterialTheme.typography.labelSmall,
                color = GlobalMaterialTheme.colorScheme.onSurface.copy(alpha = 0.45f))
            Text("300", style = GlobalMaterialTheme.typography.labelSmall,
                color = GlobalMaterialTheme.colorScheme.onSurface.copy(alpha = 0.45f))
        }
    }
}

private fun tempoLabel(bpm: Int): String = when {
    bpm < 40  -> "Grave"
    bpm < 60  -> "Largo"
    bpm < 66  -> "Larghetto"
    bpm < 76  -> "Adagio"
    bpm < 108 -> "Andante"
    bpm < 120 -> "Moderato"
    bpm < 156 -> "Allegro"
    bpm < 176 -> "Vivace"
    bpm < 200 -> "Presto"
    else      -> "Prestissimo"
}

@Preview
@Composable
private fun MetronomeContentPreview() {
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
            MetronomeContent(sheet = sampleSheet)
        }
    }
}
