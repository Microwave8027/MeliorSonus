package com.example.meliorsonus.ui.home

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Clear
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.*
import com.example.meliorsonus.theme.Miscellaneous.GlobalMaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import com.example.meliorsonus.model.SheetSearchResult

@Composable
fun LibraryContent(
    items: List<SheetSearchResult>,
    searchQuery: String,
    onSearchQueryChanged: (String) -> Unit,
    onSearchSubmit: () -> Unit,
    onItemClick: (SheetSearchResult) -> Unit,
    onAddSheet: () -> Unit,
    onDelete: (SheetSearchResult) -> Unit,
    modifier: Modifier = Modifier
) {
    val keyboardController = LocalSoftwareKeyboardController.current

    Box(modifier = modifier.fillMaxSize()) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(horizontal = 16.dp)
        ) {
            Spacer(Modifier.height(8.dp))
            Text(
                text = "My Music Library",
                style = GlobalMaterialTheme.typography.headlineMedium,
                fontWeight = FontWeight.Bold,
                color = GlobalMaterialTheme.colorScheme.onBackground
            )
            Spacer(Modifier.height(12.dp))

            OutlinedTextField(
                value = searchQuery,
                onValueChange = onSearchQueryChanged,
                placeholder = { Text("Search sheet music...") },
                leadingIcon = { Icon(Icons.Default.Search, contentDescription = "Search") },
                trailingIcon = {
                    if (searchQuery.isNotEmpty()) {
                        IconButton(onClick = { onSearchQueryChanged("") }) {
                            Icon(Icons.Default.Clear, contentDescription = "Clear")
                        }
                    }
                },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(8.dp),
                colors = OutlinedTextFieldDefaults.colors(
                    focusedBorderColor = GlobalMaterialTheme.colorScheme.primary,
                    unfocusedBorderColor = GlobalMaterialTheme.colorScheme.outline
                ),
                keyboardActions = KeyboardActions(onSearch = {
                    onSearchSubmit()
                    keyboardController?.hide()
                }),
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search)
            )

            Spacer(Modifier.height(16.dp))

            LazyColumn(
                verticalArrangement = Arrangement.spacedBy(10.dp),
                modifier = Modifier.fillMaxSize()
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

        FloatingActionButton(
            onClick = onAddSheet,
            modifier = Modifier
                .align(Alignment.BottomEnd)
                .padding(24.dp),
            containerColor = GlobalMaterialTheme.colorScheme.primary,
            contentColor = GlobalMaterialTheme.colorScheme.onPrimary
        ) {
            Icon(Icons.Default.Add, contentDescription = "Add Sheet Music")
        }
    }
}
