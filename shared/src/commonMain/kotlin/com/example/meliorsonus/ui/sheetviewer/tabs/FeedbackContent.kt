package com.example.meliorsonus.ui.sheetviewer.tabs

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Lightbulb
import androidx.compose.material.icons.filled.ThumbUp
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalInspectionMode
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.example.meliorsonus.model.SheetSearchResult
import com.example.meliorsonus.theme.Miscellaneous.GlobalMaterialTheme
import uniffi.compose_app.Card
import uniffi.compose_app.CardType
import uniffi.compose_app.test

val CardType.icon: ImageVector
    get() = when (this) {
        CardType.IMPORTANT -> Icons.Default.Warning
        CardType.TIP -> Icons.Default.Lightbulb
        CardType.FEEDBACK -> Icons.Default.ThumbUp
    }

val CardType.color: Color
    @Composable
    get() = when (this) {
        CardType.IMPORTANT -> GlobalMaterialTheme.feedbackColors.important
        CardType.TIP -> GlobalMaterialTheme.feedbackColors.tip
        CardType.FEEDBACK -> GlobalMaterialTheme.feedbackColors.feedback
    }

@Composable
fun FeedbackContent(
    sheet: SheetSearchResult,
    modifier: Modifier = Modifier
) {
    val isPreview = LocalInspectionMode.current
    val testCard: Card = remember(isPreview) {
        if (isPreview) {
            Card(
                title = "Test (Preview)",
                value = "100%",
                description = "Preview description",
                cardType = CardType.IMPORTANT
            )
        } else {
            test()
        }
    }

    Column(
        modifier = modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        FeedbackCard(
            title = testCard.title,
            value = testCard.value,
            description = testCard.description,
            cardType = testCard.cardType
        )

        // Pitch accuracy card
        FeedbackCard(
            title = "Pitch Accuracy",
            value = "87%",
            description = "Most notes are on target. Watch measure 14–16 for slight flat tendency.",
            cardType = CardType.FEEDBACK
        )

        // Rhythm card
        FeedbackCard(
            title = "Rhythm Consistency",
            value = "87%",
            description = "Excellent timing! A few syncopated passages in the bridge rushed slightly.",
            cardType = CardType.FEEDBACK
        )

        // Dynamics card
        FeedbackCard(
            title = "Dynamic Range",
            value = "Needs Work",
            description = "The forte passages lack contrast. Try exaggerating dynamics in measures 24–32.",
            cardType = CardType.IMPORTANT
        )

        // Articulation card
        FeedbackCard(
            title = "Articulation",
            value = "87%",
            description = "Legato lines are smooth. Staccato marks in the coda could be crisper.",
            cardType = CardType.TIP
        )
    }
}

@Composable
private fun FeedbackCard(
    title: String,
    value: String,
    description: String,
    cardType: CardType
) {
    val cardColor = cardType.color
    val icon = cardType.icon

    Box(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(16.dp))
            .background(GlobalMaterialTheme.colorScheme.surface)
            .border(width = 2.dp, color = cardColor.copy(alpha = 0.5f), shape = RoundedCornerShape(16.dp))
    ) {
        // Gradient tint: higher opacity near edges
        Box(
            modifier = Modifier
                .matchParentSize()
                .background(
                    Brush.radialGradient(
                        colors = listOf(
                            cardColor.copy(alpha = 0.02f),
                            cardColor.copy(alpha = 0.12f)
                        ),
                        radius = 800f
                    )
                )
        )

        Row(
            modifier = Modifier.padding(14.dp),
            verticalAlignment = Alignment.Top
        ) {
            Icon(
                imageVector = icon,
                contentDescription = null,
                tint = cardColor,
                modifier = Modifier.size(20.dp)
            )
            Spacer(Modifier.width(12.dp))
            Column {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        text = title,
                        style = GlobalMaterialTheme.typography.bodyMedium,
                        fontWeight = FontWeight.SemiBold,
                        color = GlobalMaterialTheme.colorScheme.onSurface
                    )
                    Surface(
                        color = cardColor.copy(alpha = 0.2f),
                        shape = RoundedCornerShape(6.dp),
                        border = androidx.compose.foundation.BorderStroke(1.dp, cardColor.copy(alpha = 0.5f))
                    ) {
                        Text(
                            text = value,
                            style = GlobalMaterialTheme.typography.labelSmall,
                            fontWeight = FontWeight.Bold,
                            color = cardColor,
                            modifier = Modifier.padding(horizontal = 8.dp, vertical = 2.dp)
                        )
                    }
                }
                Spacer(Modifier.height(4.dp))
                Text(
                    text = description,
                    style = GlobalMaterialTheme.typography.bodySmall,
                    color = GlobalMaterialTheme.colorScheme.onSurface.copy(alpha = 0.70f)
                )
            }
        }
    }
}

@androidx.compose.ui.tooling.preview.Preview
@Composable
private fun FeedbackContentPreview() {
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
            FeedbackContent(sheet = sampleSheet)
        }
    }
}

