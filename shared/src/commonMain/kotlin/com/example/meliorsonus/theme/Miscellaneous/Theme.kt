package com.example.meliorsonus.theme.Miscellaneous

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.ReadOnlyComposable

// Dark Mode: Background is Black (RichBlack), Text is White with slight yellow (WarmWhite)
private val DarkColorScheme = darkColorScheme(
    primary = WarmWhite,
    onPrimary = RichBlack,
    secondary = WarmWhite,
    onSecondary = RichBlack,
    tertiary = WarmWhite,
    onTertiary = RichBlack,
    background = RichBlack,
    onBackground = WarmWhite,
    surface = RichBlack,
    onSurface = WarmWhite,
    surfaceVariant = RichBlack,
    onSurfaceVariant = WarmWhite,
    outline = WarmWhite,
    outlineVariant = WarmWhite,
    error = WarmWhite,
    onError = RichBlack
)

// Light Mode: Background is White with slight yellow (WarmWhite), Text is Black (RichBlack)
private val LightColorScheme = lightColorScheme(
    primary = RichBlack,
    onPrimary = WarmWhite,
    secondary = RichBlack,
    onSecondary = WarmWhite,
    tertiary = RichBlack,
    onTertiary = WarmWhite,
    background = WarmWhite,
    onBackground = RichBlack,
    surface = WarmWhite,
    onSurface = RichBlack,
    surfaceVariant = WarmWhite,
    onSurfaceVariant = RichBlack,
    outline = RichBlack,
    outlineVariant = RichBlack,
    error = RichBlack,
    onError = WarmWhite
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
