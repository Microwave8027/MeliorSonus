package com.example.meliorsonus.ui.home.core

import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ExitToApp
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import com.arkivanov.decompose.extensions.compose.subscribeAsState
import com.example.meliorsonus.model.SheetSearchResult
import com.example.meliorsonus.theme.Miscellaneous.GlobalMaterialTheme
import com.example.meliorsonus.theme.Miscellaneous.LightPurple
import com.example.meliorsonus.ui.home.core.HomeComponent.HomeTab
import com.example.meliorsonus.ui.home.tabDirectories.HomeContent
import com.example.meliorsonus.ui.home.tabDirectories.LibraryContent
import com.example.meliorsonus.ui.home.tabDirectories.ProfileContent
import com.example.meliorsonus.ui.home.tabDirectories.SkillsContent
import kotlinx.coroutines.launch
import org.jetbrains.compose.ui.tooling.preview.Preview

/**
 * Stateless HomeScreen scaffold.
 * All state comes from [HomeComponent]; the composable never holds business logic.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HomeScreen(
    component: HomeComponent,
    modifier: Modifier = Modifier
) {
    val state by component.state.subscribeAsState()
    val snackbarHostState = remember { SnackbarHostState() }
    val drawerState = rememberDrawerState(initialValue = DrawerValue.Closed)
    val coroutineScope = rememberCoroutineScope()

    // Show error snackbar when the component emits one
    LaunchedEffect(state.errorMessage) {
        state.errorMessage?.let {
            snackbarHostState.showSnackbar(it)
        }
    }

    BoxWithConstraints(modifier = modifier.fillMaxSize()) {
        val isTablet = maxWidth >= 600.dp

        CompositionLocalProvider(LocalLayoutDirection provides LayoutDirection.Rtl) {
            ModalNavigationDrawer(
                drawerState = drawerState,
                drawerContent = {
                    CompositionLocalProvider(LocalLayoutDirection provides LayoutDirection.Ltr) {
                        ModalDrawerSheet(
                            modifier = Modifier
                                .width(280.dp)
                                .fillMaxHeight(),
                            drawerContainerColor = GlobalMaterialTheme.colorScheme.surface,
                            drawerTonalElevation = 2.dp
                        ) {
                            Spacer(Modifier.height(24.dp))
                            Text(
                                text = "Preferences & Settings",
                                style = GlobalMaterialTheme.typography.titleMedium,
                                fontWeight = FontWeight.Bold,
                                modifier = Modifier.padding(horizontal = 20.dp, vertical = 8.dp),
                                color = GlobalMaterialTheme.colorScheme.onSurface
                            )
                            HorizontalDivider(
                                modifier = Modifier.padding(vertical = 12.dp, horizontal = 20.dp),
                                color = GlobalMaterialTheme.colorScheme.outlineVariant
                            )
                            NavigationDrawerItem(
                                icon = { Icon(Icons.Default.Settings, contentDescription = "Settings") },
                                label = { Text("App Preferences") },
                                selected = false,
                                onClick = { coroutineScope.launch { drawerState.close() } },
                                modifier = Modifier.padding(horizontal = 12.dp)
                            )
                            NavigationDrawerItem(
                                icon = { Icon(Icons.Default.Info, contentDescription = "About") },
                                label = { Text("About MeliorSonus") },
                                selected = false,
                                onClick = { coroutineScope.launch { drawerState.close() } },
                                modifier = Modifier.padding(horizontal = 12.dp)
                            )
                            NavigationDrawerItem(
                                icon = { Icon(Icons.AutoMirrored.Filled.ExitToApp, contentDescription = "Logout") },
                                label = { Text("Logout") },
                                selected = false,
                                onClick = { coroutineScope.launch { drawerState.close() } },
                                modifier = Modifier.padding(horizontal = 12.dp)
                            )
                        }
                    }
                }
            ) {
                CompositionLocalProvider(LocalLayoutDirection provides LayoutDirection.Ltr) {
                    Scaffold(
                        snackbarHost = { SnackbarHost(hostState = snackbarHostState) },
                        topBar = {
                            TopAppBar(
                                title = {
                                    Text(
                                        text = "MeliorSonus",
                                        fontWeight = FontWeight.Bold,
                                        color = GlobalMaterialTheme.colorScheme.onSurface
                                    )
                                },
                                actions = {
                                    IconButton(onClick = {
                                        coroutineScope.launch {
                                            if (drawerState.isClosed) drawerState.open() else drawerState.close()
                                        }
                                    }) {
                                        Icon(imageVector = Icons.Default.Menu, contentDescription = "Right Menu")
                                    }
                                },
                                colors = TopAppBarDefaults.topAppBarColors(
                                    containerColor = GlobalMaterialTheme.colorScheme.surface
                                )
                            )
                        },
                        bottomBar = {
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

                                NavigationBarItem(
                                    icon = { Icon(Icons.Default.Home, contentDescription = "Home") },
                                    label = { Text("Home") },
                                    selected = state.activeTab == HomeTab.HOME,
                                    onClick = { component.onTabSelected(HomeTab.HOME) },
                                    colors = navItemColors
                                )
                                NavigationBarItem(
                                    icon = { Icon(Icons.Default.Star, contentDescription = "Skills") },
                                    label = { Text("Skills") },
                                    selected = state.activeTab == HomeTab.SKILLS,
                                    onClick = { component.onTabSelected(HomeTab.SKILLS) },
                                    colors = navItemColors
                                )
                                NavigationBarItem(
                                    icon = { Icon(Icons.Default.Search, contentDescription = "Search") },
                                    label = { Text("Search") },
                                    selected = state.activeTab == HomeTab.LIBRARY,
                                    onClick = { component.onTabSelected(HomeTab.LIBRARY) },
                                    colors = navItemColors
                                )
                                NavigationBarItem(
                                    icon = { Icon(Icons.Default.Person, contentDescription = "Profile") },
                                    label = { Text("Profile") },
                                    selected = state.activeTab == HomeTab.PROFILE,
                                    onClick = { component.onTabSelected(HomeTab.PROFILE) },
                                    colors = navItemColors
                                )
                            }
                        }
                    ) { innerPadding ->
                        Box(
                            modifier = Modifier
                                .fillMaxSize()
                                .padding(innerPadding)
                        ) {
                            when (state.activeTab) {
                                HomeTab.HOME -> HomeContent(
                                    items = state.sheetMusicList,
                                    onItemClick = component::onOpenSheetViewer,
                                    onDelete = component::onDeleteSheet
                                )
                                HomeTab.SKILLS -> SkillsContent()
                                HomeTab.LIBRARY -> LibraryContent(
                                    isTablet = isTablet,
                                    query = state.searchPopupQuery,
                                    onQueryChanged = component::onSearchPopupQueryChanged,
                                    instrumentQuery = state.searchPopupInstrument,
                                    onInstrumentQueryChanged = component::onSearchPopupInstrumentChanged,
                                    onSearchSubmit = component::onSearchPopupSubmit,
                                    isLoading = state.isSearchPopupLoading,
                                    searchError = state.searchPopupError,
                                    results = state.searchPopupResults,
                                    selectedResult = state.selectedSearchResult,
                                    pdfPath = state.pdfPath,
                                    isFetchingPdf = state.isFetchingPdf,
                                    pdfFetchError = state.pdfFetchError,
                                    isSaving = state.isSaving,
                                    onResultClick = component::onSearchResultSelected,
                                    onClearSelection = component::onClearSearchSelection,
                                    onConfirmAndSave = component::onConfirmAndSaveSheet,
                                    loadMore = component::loadMore,
                                    isFetchingMoreSheets = state.isFetchingMoreSheets,
                                    loadMoreError = state.loadMoreError
                                )
                                HomeTab.PROFILE -> ProfileContent()
                            }
                        }
                    }
                }
            }
        }

    }
}

private val sampleSheetsForPreview = listOf(
    SheetSearchResult(
        mxl = "sample1.mxl",
        pdf = "sample1.pdf",
        title = "Moonlight Sonata",
        composer = "Ludwig van Beethoven",
        songLengthBars = 64,
        genres = "Classical",
        instruments = emptyList()
    ),
    SheetSearchResult(
        mxl = "sample2.mxl",
        pdf = "sample2.pdf",
        title = "Clair de Lune",
        composer = "Claude Debussy",
        songLengthBars = 72,
        genres = "Impressionism",
        instruments = emptyList()
    ),
    SheetSearchResult(
        mxl = "sample3.mxl",
        pdf = "sample3.pdf",
        title = "Für Elise",
        composer = "Ludwig van Beethoven",
        songLengthBars = 48,
        genres = "Classical",
        instruments = emptyList()
    )
)

@Preview
@Composable
private fun HomeScreenLightPreview() {
    GlobalMaterialTheme(darkTheme = false) {
        Surface(modifier = Modifier.fillMaxSize()) {
            HomeContent(
                items = sampleSheetsForPreview,
                onItemClick = {},
                onDelete = {}
            )
        }
    }
}

@Preview
@Composable
private fun HomeScreenDarkPreview() {
    GlobalMaterialTheme(darkTheme = true) {
        Surface(modifier = Modifier.fillMaxSize()) {
            HomeContent(
                items = sampleSheetsForPreview,
                onItemClick = {},
                onDelete = {}
            )
        }
    }
}

