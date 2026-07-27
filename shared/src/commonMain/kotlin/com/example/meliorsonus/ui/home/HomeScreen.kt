package com.example.meliorsonus.ui.home

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ExitToApp
import androidx.compose.material.icons.automirrored.filled.List
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import com.arkivanov.decompose.extensions.compose.subscribeAsState
import com.example.meliorsonus.theme.Miscellaneous.GlobalMaterialTheme
import com.example.meliorsonus.ui.home.HomeComponent.HomeTab
import kotlinx.coroutines.launch

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
                                containerColor = GlobalMaterialTheme.colorScheme.onTertiary,
                                tonalElevation = 8.dp
                            ) {
                                NavigationBarItem(
                                    icon = { Icon(Icons.Default.Home, contentDescription = "Home") },
                                    label = { Text("Home") },
                                    selected = state.activeTab == HomeTab.HOME,
                                    onClick = { component.onTabSelected(HomeTab.HOME) }
                                )
                                NavigationBarItem(
                                    icon = { Icon(Icons.Default.Star, contentDescription = "Skills") },
                                    label = { Text("Skills") },
                                    selected = state.activeTab == HomeTab.SKILLS,
                                    onClick = { component.onTabSelected(HomeTab.SKILLS) }
                                )
                                NavigationBarItem(
                                    icon = { Icon(Icons.AutoMirrored.Filled.List, contentDescription = "Library") },
                                    label = { Text("Library") },
                                    selected = state.activeTab == HomeTab.LIBRARY,
                                    onClick = { component.onTabSelected(HomeTab.LIBRARY) }
                                )
                                NavigationBarItem(
                                    icon = { Icon(Icons.Default.Person, contentDescription = "Profile") },
                                    label = { Text("Profile") },
                                    selected = state.activeTab == HomeTab.PROFILE,
                                    onClick = { component.onTabSelected(HomeTab.PROFILE) }
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
                                    items = state.sheetMusicList,
                                    searchQuery = state.librarySearchQuery,
                                    onSearchQueryChanged = component::onLibrarySearchQueryChanged,
                                    onSearchSubmit = component::onLibrarySearchSubmit,
                                    onItemClick = component::onOpenSheetViewer,
                                    onAddSheet = component::onOpenAddSheetPopup,
                                    onDelete = component::onDeleteSheet
                                )
                                HomeTab.PROFILE -> ProfileContent()
                            }
                        }
                    }
                }
            }
        }

        // Add-sheet popup — tablet vs phone adaptive
        if (state.showAddSheetPopup) {
            if (isTablet) {
                Box(
                    modifier = Modifier
                        .fillMaxSize()
                        .background(Color.Black.copy(alpha = 0.5f)),
                    contentAlignment = Alignment.Center
                ) {
                    Card(
                        modifier = Modifier
                            .width(600.dp)
                            .height(700.dp),
                        shape = RoundedCornerShape(16.dp),
                        colors = CardDefaults.cardColors(containerColor = GlobalMaterialTheme.colorScheme.surface),
                        elevation = CardDefaults.cardElevation(defaultElevation = 8.dp)
                    ) {
                        AddSheetPopupContent(
                            isTablet = true,
                            query = state.searchPopupQuery,
                            onQueryChanged = component::onSearchPopupQueryChanged,
                            onSearchSubmit = component::onSearchPopupSubmit,
                            isLoading = state.isSearchPopupLoading,
                            searchError = state.searchPopupError,
                            results = state.searchPopupResults,
                            selectedResult = state.selectedSearchResult,
                            pdfPath = state.pdfPath,
                            isFetchingPdf = state.isFetchingPdf,
                            pdfFetchError = state.pdfFetchError,
                            showZoomChecker = state.showZoomChecker,
                            zoomLevel = state.zoomLevel,
                            isSaving = state.isSaving,
                            onZoomLevelChanged = component::onZoomLevelChanged,
                            onBackFromZoomChecker = component::onBackFromZoomChecker,
                            onConfirmZoomAndProceed = component::onConfirmZoomAndProceed,
                            onResultClick = component::onSearchResultSelected,
                            onClearSelection = component::onClearSearchSelection,
                            onProceed = component::onProceedToZoomChecker,
                            onClose = component::onCloseAddSheetPopup,
                            verovioComponent = component.verovioManagerComponent
                        )
                    }
                }
            } else {
                ModalBottomSheet(
                    onDismissRequest = component::onCloseAddSheetPopup,
                    dragHandle = { BottomSheetDefaults.DragHandle() },
                    shape = RoundedCornerShape(topStart = 16.dp, topEnd = 16.dp)
                ) {
                    Box(
                        modifier = Modifier
                            .fillMaxWidth()
                            .fillMaxHeight(0.9f)
                    ) {
                        AddSheetPopupContent(
                            isTablet = false,
                            query = state.searchPopupQuery,
                            onQueryChanged = component::onSearchPopupQueryChanged,
                            onSearchSubmit = component::onSearchPopupSubmit,
                            isLoading = state.isSearchPopupLoading,
                            searchError = state.searchPopupError,
                            results = state.searchPopupResults,
                            selectedResult = state.selectedSearchResult,
                            pdfPath = state.pdfPath,
                            isFetchingPdf = state.isFetchingPdf,
                            pdfFetchError = state.pdfFetchError,
                            showZoomChecker = state.showZoomChecker,
                            zoomLevel = state.zoomLevel,
                            isSaving = state.isSaving,
                            onZoomLevelChanged = component::onZoomLevelChanged,
                            onBackFromZoomChecker = component::onBackFromZoomChecker,
                            onConfirmZoomAndProceed = component::onConfirmZoomAndProceed,
                            onResultClick = component::onSearchResultSelected,
                            onClearSelection = component::onClearSearchSelection,
                            onProceed = component::onProceedToZoomChecker,
                            onClose = component::onCloseAddSheetPopup,
                            verovioComponent = component.verovioManagerComponent
                        )
                    }
                }
            }
        }
    }
}
