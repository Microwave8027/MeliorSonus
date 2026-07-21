package com.example.meliorsonus.ui.home

import androidx.compose.foundation.Image
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material3.*
import com.example.meliorsonus.theme.Miscellaneous.GlobalMaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import meliorsonus.shared.generated.resources.Res
import meliorsonus.shared.generated.resources.music_cover_placeholder
import org.jetbrains.compose.resources.painterResource
import androidx.compose.ui.unit.dp
import com.example.meliorsonus.model.SheetSearchResult

@Composable
fun HomeContent(
    items: List<SheetSearchResult>,
    onItemClick: (SheetSearchResult) -> Unit,
    onDelete: (SheetSearchResult) -> Unit,
    modifier: Modifier = Modifier
) {
    Column(
        modifier = modifier
            .fillMaxSize()
            .padding(horizontal = 16.dp)
    ) {
        Spacer(Modifier.height(8.dp))
        Text(
            text = "Welcome Back",
            style = GlobalMaterialTheme.typography.headlineMedium,
            fontWeight = FontWeight.Bold,
            color = GlobalMaterialTheme.colorScheme.onBackground
        )
        Text(
            text = "Continue your music practice session",
            style = GlobalMaterialTheme.typography.bodyMedium,
            color = GlobalMaterialTheme.colorScheme.onSurfaceVariant
        )
        Spacer(Modifier.height(16.dp))

        Text(
            text = "Recently Played",
            style = GlobalMaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.Bold,
            modifier = Modifier.padding(bottom = 12.dp),
            color = GlobalMaterialTheme.colorScheme.onBackground
        )

        LazyColumn(
            verticalArrangement = Arrangement.spacedBy(10.dp),
            modifier = Modifier.weight(1f)
        ) {
            items(items, key = { it.mxl }) { item ->
                SwipeToDeleteContainer(
                    onDelete = { onDelete(item) }
                ) {
                    MusicRowItem(
                        item = item,
                        onClick = { onItemClick(item) },
                        onDelete = { onDelete(item) }
                    )
                }
            }
        }
    }
}

@Composable
fun MusicRowItem(
    item: SheetSearchResult,
    onClick: (() -> Unit)? = null,
    onDelete: (() -> Unit)? = null,
    modifier: Modifier = Modifier
) {
    Card(
        modifier = modifier
            .fillMaxWidth()
            .then(if (onClick != null) Modifier.clickable { onClick() } else Modifier),
        colors = CardDefaults.cardColors(containerColor = GlobalMaterialTheme.colorScheme.surface),
        shape = RoundedCornerShape(8.dp)
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(12.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            MusicSheetThumbnail(
                modifier = Modifier
                    .size(50.dp)
                    .clip(RoundedCornerShape(6.dp))
                    .background(GlobalMaterialTheme.colorScheme.background)
            )
            Spacer(Modifier.width(16.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = item.title,
                    style = GlobalMaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.SemiBold,
                    color = GlobalMaterialTheme.colorScheme.onSurface,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis
                )
                Text(
                    text = "${item.composerName} • ${item.songLengthBars} bars",
                    style = GlobalMaterialTheme.typography.bodySmall,
                    color = GlobalMaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis
                )
            }
            Spacer(Modifier.width(8.dp))
            if (item.genres.isNotEmpty()) {
                Surface(
                    color = GlobalMaterialTheme.colorScheme.primaryContainer,
                    shape = RoundedCornerShape(4.dp)
                ) {
                    Text(
                        text = item.genres,
                        style = GlobalMaterialTheme.typography.bodySmall,
                        fontWeight = FontWeight.Medium,
                        color = GlobalMaterialTheme.colorScheme.onPrimaryContainer,
                        modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp)
                    )
                }
            }
        }
    }
}

@Composable
fun MusicSheetThumbnail(modifier: Modifier = Modifier) {
    Image(
        painter = painterResource(Res.drawable.music_cover_placeholder),
        contentDescription = "Music Sheet Cover",
        modifier = modifier,
        contentScale = ContentScale.Crop
    )
}

