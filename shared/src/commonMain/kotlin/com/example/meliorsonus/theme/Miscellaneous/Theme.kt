package com.example.meliorsonus.theme.Miscellaneous

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.ReadOnlyComposable
import androidx.compose.ui.graphics.Color

private val DarkColorScheme = darkColorScheme(
    primary = GroovAmber,
    onPrimary = GroovDeepBlack,
    secondary = GroovCoral,
    onSecondary = GroovDeepBlack,
    tertiary = GroovLavender,
    onTertiary = GroovDeepBlack,
    background = GroovCharcoal,
    onBackground = GroovCream,
    surface = GroovCharcoal,
    onSurface = GroovCream,
    surfaceVariant = GroovSurface,
    onSurfaceVariant = GroovMuted,
    outline = GroovBorder,
    outlineVariant = GroovMuted,
    error = GroovError,
    onError = Color.White
)

private val LightColorScheme = lightColorScheme(
    primary = GroovAmber,
    onPrimary = GroovDeepBlack,
    secondary = GroovCoral,
    onSecondary = Color.White,
    tertiary = GroovLavender,
    onTertiary = Color.White,
    background = GroovLightBackground,
    onBackground = GroovLightOnBackground,
    surface = GroovLightSurface,
    onSurface = GroovLightOnBackground,
    surfaceVariant = Color(0xFFF5F0E8),
    onSurfaceVariant = Color(0xFF6E6E73),
    outline = Color(0xFFD1C4A9),
    outlineVariant = Color(0xFF9E9E9E),
    error = GroovError,
    onError = Color.White
)

@Composable
fun GlobalMaterialTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    content: @Composable () -> Unit
) {
    val colorScheme = if (darkTheme) DarkColorScheme else LightColorScheme

    MaterialTheme(
        colorScheme = colorScheme,
        typography = MeliorSonusTypography,
        content = content
    )
}

object GlobalMaterialTheme {
    val colorScheme: ColorScheme
        @Composable
        @ReadOnlyComposable
        get() = MaterialTheme.colorScheme

    val typography: Typography
        @Composable
        @ReadOnlyComposable
        get() = MaterialTheme.typography

    val shapes: Shapes
        @Composable
        @ReadOnlyComposable
        get() = MaterialTheme.shapes
}
