package com.example.meliorsonus.ui.sheetviewer.core

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.FormatListBulleted
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.arkivanov.decompose.extensions.compose.subscribeAsState
import com.example.meliorsonus.theme.Miscellaneous.GlobalMaterialTheme
import com.example.meliorsonus.theme.Miscellaneous.LightPurple
import com.example.meliorsonus.theme.Miscellaneous.MildPurple
import com.example.meliorsonus.ui.sheetviewer.tabs.FeedbackContent
import com.example.meliorsonus.ui.sheetviewer.tabs.MetronomeContent
import com.example.meliorsonus.ui.sheetviewer.tabs.ModesContent
import com.example.meliorsonus.ui.sheetviewer.tabs.StatisticsContent
import com.example.meliorsonus.ui.sheetviewer.tabs.SummaryContent

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SheetViewerScreen(
    component: SheetViewerComponent,
    modifier: Modifier = Modifier
) {
    val state by component.state.subscribeAsState()
    val sheet = state.sheet
    val selectedTab = state.selectedTab

    Scaffold(
        modifier = modifier.fillMaxSize(),
        topBar = {
            Column {
                // --- Top App Bar: sheet info ---
                TopAppBar(
                    title = {
                        Column {
                            Text(
                                text = sheet.title,
                                style = GlobalMaterialTheme.typography.titleMedium,
                                fontWeight = FontWeight.Bold,
                                color = GlobalMaterialTheme.colorScheme.onSurface
                            )
                            Text(
                                text = "${sheet.composer} • ${sheet.songLengthBars} bars • ${sheet.genres}",
                                style = GlobalMaterialTheme.typography.bodySmall,
                                color = GlobalMaterialTheme.colorScheme.onSurface.copy(alpha = 0.70f)
                            )
                        }
                    },
                    navigationIcon = {
                        IconButton(onClick = component::onBack) {
                            Icon(
                                imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                                contentDescription = "Back",
                                tint = GlobalMaterialTheme.colorScheme.onSurface
                            )
                        }
                    },
                    actions = {
                        IconButton(onClick = { /* Settings – not yet implemented */ }) {
                            Icon(
                                imageVector = Icons.Default.Settings,
                                contentDescription = "Settings",
                                tint = GlobalMaterialTheme.colorScheme.onSurface
                            )
                        }
                    },
                    colors = TopAppBarDefaults.topAppBarColors(
                        containerColor = GlobalMaterialTheme.colorScheme.surface
                    )
                )
                // --- Divider between top bar and content ---
                HorizontalDivider(
                    color = GlobalMaterialTheme.colorScheme.onSurface.copy(alpha = 0.12f),
                    thickness = 1.dp
                )
                // --- AI Search Bar ---
                AiSearchBar(
                    query = state.aiQuery,
                    onQueryChanged = component::onAiQueryChanged,
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 16.dp, vertical = 10.dp)
                )
                // --- Active tab label ---
                Text(
                    text = selectedTab.displayName(),
                    style = GlobalMaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold,
                    color = GlobalMaterialTheme.colorScheme.onBackground,
                    modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp)
                )
                Spacer(Modifier.height(4.dp))
            }
        },
        bottomBar = {
            Column {
                // --- Divider between content and bottom bar ---
                HorizontalDivider(
                    color = GlobalMaterialTheme.colorScheme.onSurface.copy(alpha = 0.12f),
                    thickness = 1.dp
                )
                NavigationBar(
                    containerColor = GlobalMaterialTheme.colorScheme.surface,
                    tonalElevation = 0.dp
                ) {
                    val navItemColors = NavigationBarItemDefaults.colors(
                        selectedIconColor = LightPurple,
                        selectedTextColor = LightPurple,
                        unselectedIconColor = GlobalMaterialTheme.colorScheme.onSurfaceVariant,
                        unselectedTextColor = GlobalMaterialTheme.colorScheme.onSurfaceVariant,
                        indicatorColor = LightPurple.copy(alpha = 0.2f)
                    )

                    ViewerTab.entries.forEach { tab ->
                        NavigationBarItem(
                            icon = {
                                Icon(
                                    imageVector = tab.icon(),
                                    contentDescription = tab.displayName()
                                )
                            },
                            label = { Text(tab.displayName()) },
                            selected = selectedTab == tab,
                            onClick = { component.onTabSelected(tab) },
                            colors = navItemColors
                        )
                    }
                }
            }
        }
    ) { paddingValues ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
        ) {
            when (selectedTab) {
                ViewerTab.FEEDBACK    -> FeedbackContent(sheet = sheet)
                ViewerTab.STATISTICS  -> StatisticsContent(sheet = sheet)
                ViewerTab.MODES       -> ModesContent(sheet = sheet)
                ViewerTab.SUMMARY     -> SummaryContent(sheet = sheet)
                ViewerTab.METRONOME   -> MetronomeContent(sheet = sheet)
            }
        }
    }
}


@Composable
private fun AiSearchBar(
    query: String,
    onQueryChanged: (String) -> Unit,
    modifier: Modifier = Modifier
) {
    val colors = GlobalMaterialTheme.colorScheme
    val isDark = colors.background.red < 0.5f

    Box(
        modifier = modifier
            .height(48.dp)
            .clip(RoundedCornerShape(24.dp))
            .background(colors.surface.copy(alpha = 0.85f))
            .then(
                Modifier.background(
                    color = colors.onSurface.copy(alpha = 0.06f),
                    shape = RoundedCornerShape(24.dp)
                )
            ),
        contentAlignment = Alignment.CenterStart
    ) {
        Row(
            modifier = Modifier
                .fillMaxSize()
                .padding(horizontal = 8.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            // Left "Ask" pill badge
            Box(
                modifier = Modifier
                    .clip(RoundedCornerShape(16.dp))
                    .background(
                        if (isDark) MildPurple.copy(alpha = 0.85f)
                        else LightPurple.copy(alpha = 0.85f)
                    )
                    .padding(horizontal = 10.dp, vertical = 4.dp),
                contentAlignment = Alignment.Center
            ) {
                Text(
                    text = "Ask",
                    color = androidx.compose.ui.graphics.Color.White,
                    fontWeight = FontWeight.SemiBold,
                    fontSize = 12.sp
                )
            }
            Spacer(Modifier.width(8.dp))
            // Magnifying glass icon
            Icon(
                imageVector = Icons.Default.Search,
                contentDescription = null,
                tint = colors.onSurface.copy(alpha = 0.45f),
                modifier = Modifier.size(18.dp)
            )
            Spacer(Modifier.width(6.dp))
            // Actual text field
            Box(modifier = Modifier.weight(1f)) {
                if (query.isEmpty()) {
                    Text(
                        text = "Ask AI",
                        color = colors.onSurface.copy(alpha = 0.40f),
                        style = GlobalMaterialTheme.typography.bodyMedium
                    )
                }
                BasicTextField(
                    value = query,
                    onValueChange = onQueryChanged,
                    singleLine = true,
                    textStyle = GlobalMaterialTheme.typography.bodyMedium.copy(
                        color = colors.onSurface
                    ),
                    cursorBrush = SolidColor(LightPurple),
                    modifier = Modifier.fillMaxWidth()
                )
            }
        }
    }
}


fun ViewerTab.displayName(): String = when (this) {
    ViewerTab.FEEDBACK   -> "Feedback"
    ViewerTab.STATISTICS -> "Statistics"
    ViewerTab.MODES      -> "Modes"
    ViewerTab.SUMMARY    -> "Summary"
    ViewerTab.METRONOME  -> "Metronome"
}

fun ViewerTab.icon(): ImageVector = when (this) {
    ViewerTab.FEEDBACK   -> Icons.Default.MusicNote
    ViewerTab.STATISTICS -> Icons.Default.BarChart
    ViewerTab.MODES      -> Icons.Default.Tune
    ViewerTab.SUMMARY    -> Icons.AutoMirrored.Filled.FormatListBulleted
    ViewerTab.METRONOME  -> Icons.Default.Timer
}


private val previewSheet = com.example.meliorsonus.model.SheetSearchResult(
    mxl = "preview.mxl",
    pdf = "preview.pdf",
    title = "Moonlight Sonata",
    composer = "L. v. Beethoven",
    songLengthBars = 64,
    genres = "Classical",
    instruments = listOf("Piano")
)

@org.jetbrains.compose.ui.tooling.preview.Preview
@Composable
private fun SheetViewerScreenLightPreview() {
    GlobalMaterialTheme(darkTheme = false) {
        Surface(modifier = Modifier.fillMaxSize()) {
            val state = SheetViewerComponent.SheetViewerState(sheet = previewSheet)
            Scaffold(
                topBar = {
                    Column {
                        TopAppBar(
                            title = {
                                Column {
                                    Text(
                                        text = state.sheet.title,
                                        style = GlobalMaterialTheme.typography.titleMedium,
                                        fontWeight = FontWeight.Bold,
                                        color = GlobalMaterialTheme.colorScheme.onSurface
                                    )
                                    Text(
                                        text = "${state.sheet.composer} • ${state.sheet.songLengthBars} bars • ${state.sheet.genres}",
                                        style = GlobalMaterialTheme.typography.bodySmall,
                                        color = GlobalMaterialTheme.colorScheme.onSurface.copy(alpha = 0.70f)
                                    )
                                }
                            },
                            navigationIcon = {
                                IconButton(onClick = {}) {
                                    Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back",
                                        tint = GlobalMaterialTheme.colorScheme.onSurface)
                                }
                            },
                            actions = {
                                IconButton(onClick = {}) {
                                    Icon(Icons.Default.Settings, contentDescription = "Settings",
                                        tint = GlobalMaterialTheme.colorScheme.onSurface)
                                }
                            },
                            colors = TopAppBarDefaults.topAppBarColors(
                                containerColor = GlobalMaterialTheme.colorScheme.surface
                            )
                        )
                        HorizontalDivider(color = GlobalMaterialTheme.colorScheme.onSurface.copy(alpha = 0.12f), thickness = 1.dp)
                        AiSearchBar(query = "", onQueryChanged = {},
                            modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 10.dp))
                        Text(text = state.selectedTab.displayName(),
                            style = GlobalMaterialTheme.typography.titleMedium,
                            fontWeight = FontWeight.Bold,
                            color = GlobalMaterialTheme.colorScheme.onBackground,
                            modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp))
                        Spacer(Modifier.height(4.dp))
                    }
                },
                bottomBar = {
                    Column {
                        HorizontalDivider(color = GlobalMaterialTheme.colorScheme.onSurface.copy(alpha = 0.12f), thickness = 1.dp)
                        NavigationBar(containerColor = GlobalMaterialTheme.colorScheme.surface, tonalElevation = 0.dp) {
                            val navColors = NavigationBarItemDefaults.colors(
                                selectedIconColor = LightPurple, selectedTextColor = LightPurple,
                                unselectedIconColor = GlobalMaterialTheme.colorScheme.onSurfaceVariant,
                                unselectedTextColor = GlobalMaterialTheme.colorScheme.onSurfaceVariant,
                                indicatorColor = LightPurple.copy(alpha = 0.2f)
                            )
                            ViewerTab.entries.forEach { tab ->
                                NavigationBarItem(
                                    icon = { Icon(tab.icon(), contentDescription = tab.displayName()) },
                                    label = { Text(tab.displayName()) },
                                    selected = state.selectedTab == tab,
                                    onClick = {},
                                    colors = navColors
                                )
                            }
                        }
                    }
                }
            ) { padding ->
                Box(Modifier.fillMaxSize().padding(padding)) {
                    FeedbackContent(sheet = previewSheet)
                }
            }
        }
    }
}

@org.jetbrains.compose.ui.tooling.preview.Preview
@Composable
private fun SheetViewerScreenDarkPreview() {
    GlobalMaterialTheme(darkTheme = true) {
        Surface(modifier = Modifier.fillMaxSize()) {
            val state = SheetViewerComponent.SheetViewerState(
                sheet = previewSheet,
                selectedTab = ViewerTab.STATISTICS
            )
            Scaffold(
                topBar = {
                    Column {
                        TopAppBar(
                            title = {
                                Column {
                                    Text(
                                        text = state.sheet.title,
                                        style = GlobalMaterialTheme.typography.titleMedium,
                                        fontWeight = FontWeight.Bold,
                                        color = GlobalMaterialTheme.colorScheme.onSurface
                                    )
                                    Text(
                                        text = "${state.sheet.composer} • ${state.sheet.songLengthBars} bars • ${state.sheet.genres}",
                                        style = GlobalMaterialTheme.typography.bodySmall,
                                        color = GlobalMaterialTheme.colorScheme.onSurface.copy(alpha = 0.70f)
                                    )
                                }
                            },
                            navigationIcon = {
                                IconButton(onClick = {}) {
                                    Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back",
                                        tint = GlobalMaterialTheme.colorScheme.onSurface)
                                }
                            },
                            actions = {
                                IconButton(onClick = {}) {
                                    Icon(Icons.Default.Settings, contentDescription = "Settings",
                                        tint = GlobalMaterialTheme.colorScheme.onSurface)
                                }
                            },
                            colors = TopAppBarDefaults.topAppBarColors(
                                containerColor = GlobalMaterialTheme.colorScheme.surface
                            )
                        )
                        HorizontalDivider(color = GlobalMaterialTheme.colorScheme.onSurface.copy(alpha = 0.12f), thickness = 1.dp)
                        AiSearchBar(query = "How do I improve my vibrato?", onQueryChanged = {},
                            modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 10.dp))
                        Text(text = state.selectedTab.displayName(),
                            style = GlobalMaterialTheme.typography.titleMedium,
                            fontWeight = FontWeight.Bold,
                            color = GlobalMaterialTheme.colorScheme.onBackground,
                            modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp))
                        Spacer(Modifier.height(4.dp))
                    }
                },
                bottomBar = {
                    Column {
                        HorizontalDivider(color = GlobalMaterialTheme.colorScheme.onSurface.copy(alpha = 0.12f), thickness = 1.dp)
                        NavigationBar(containerColor = GlobalMaterialTheme.colorScheme.surface, tonalElevation = 0.dp) {
                            val navColors = NavigationBarItemDefaults.colors(
                                selectedIconColor = LightPurple, selectedTextColor = LightPurple,
                                unselectedIconColor = GlobalMaterialTheme.colorScheme.onSurfaceVariant,
                                unselectedTextColor = GlobalMaterialTheme.colorScheme.onSurfaceVariant,
                                indicatorColor = LightPurple.copy(alpha = 0.2f)
                            )
                            ViewerTab.entries.forEach { tab ->
                                NavigationBarItem(
                                    icon = { Icon(tab.icon(), contentDescription = tab.displayName()) },
                                    label = { Text(tab.displayName()) },
                                    selected = state.selectedTab == tab,
                                    onClick = {},
                                    colors = navColors
                                )
                            }
                        }
                    }
                }
            ) { padding ->
                Box(Modifier.fillMaxSize().padding(padding)) {
                    StatisticsContent(sheet = previewSheet)
                }
            }
        }
    }
}
