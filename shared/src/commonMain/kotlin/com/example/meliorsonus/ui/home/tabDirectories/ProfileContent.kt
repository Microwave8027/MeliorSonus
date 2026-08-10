package com.example.meliorsonus.ui.home.tabDirectories

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Person
import androidx.compose.material3.*
import com.example.meliorsonus.theme.Miscellaneous.GlobalMaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp

@Composable
fun ProfileContent(modifier: Modifier = Modifier) {
    Column(
        modifier = modifier
            .fillMaxSize()
            .padding(24.dp),
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        Spacer(Modifier.height(16.dp))
        Box(
            modifier = Modifier
                .size(96.dp)
                .clip(RoundedCornerShape(48.dp))
                .background(GlobalMaterialTheme.colorScheme.secondary),
            contentAlignment = Alignment.Center
        ) {
            Icon(
                imageVector = Icons.Default.Person,
                contentDescription = "Avatar",
                modifier = Modifier.size(48.dp),
                tint = GlobalMaterialTheme.colorScheme.onSecondary
            )
        }
        Spacer(Modifier.height(16.dp))
        Text(
            text = "Music Practitioner",
            style = GlobalMaterialTheme.typography.headlineSmall,
            fontWeight = FontWeight.Bold,
            color = GlobalMaterialTheme.colorScheme.onBackground
        )
        Text(
            text = "level 4 • Intermediate Musician",
            style = GlobalMaterialTheme.typography.bodyMedium,
            color = GlobalMaterialTheme.colorScheme.primary
        )
        Spacer(Modifier.height(32.dp))

        Card(
            modifier = Modifier.fillMaxWidth(),
            colors = CardDefaults.cardColors(containerColor = GlobalMaterialTheme.colorScheme.surface)
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Text(
                    text = "Weekly Analytics",
                    style = GlobalMaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold,
                    color = GlobalMaterialTheme.colorScheme.onSurface
                )
                Spacer(Modifier.height(12.dp))
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween
                ) {
                    Text("Practiced Minutes", color = GlobalMaterialTheme.colorScheme.onSurfaceVariant)
                    Text("180 min", fontWeight = FontWeight.Bold)
                }
                Spacer(Modifier.height(8.dp))
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween
                ) {
                    Text("Average Pitch Accuracy", color = GlobalMaterialTheme.colorScheme.onSurfaceVariant)
                    Text("87 %", fontWeight = FontWeight.Bold, color = GlobalMaterialTheme.colorScheme.primary)
                }
            }
        }
    }
}
