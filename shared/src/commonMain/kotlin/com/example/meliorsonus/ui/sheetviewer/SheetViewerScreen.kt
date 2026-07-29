package com.example.meliorsonus.ui.sheetviewer

import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.arkivanov.decompose.extensions.compose.subscribeAsState
import com.example.meliorsonus.theme.Miscellaneous.GlobalMaterialTheme

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SheetViewerScreen(
    component: SheetViewerComponent,
    modifier: Modifier = Modifier
) {
    val state by component.state.subscribeAsState()
    val sheet = state.sheet

    Scaffold(
        modifier = Modifier.fillMaxSize(),
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Text(
                            text = sheet.title,
                            style = GlobalMaterialTheme.typography.titleMedium,
                            color = GlobalMaterialTheme.colorScheme.onSurface
                        )
                        Text(
                            text = sheet.composerName,
                            style = GlobalMaterialTheme.typography.bodySmall,
                            color = GlobalMaterialTheme.colorScheme.onSurface.copy(alpha = 0.75f)
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
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = GlobalMaterialTheme.colorScheme.surface
                )
            )
        },
    ) { paddingValues ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues),
            contentAlignment = Alignment.Center
        ) {
            Text(
                text = "Viewer not implemented",
                style = GlobalMaterialTheme.typography.bodyLarge
            )
        }
    }
}
