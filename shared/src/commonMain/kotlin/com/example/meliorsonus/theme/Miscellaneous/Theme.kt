package com.example.meliorsonus.theme.Miscellaneous

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.ReadOnlyComposable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.staticCompositionLocalOf

import androidx.compose.ui.graphics.Color

data class FeedbackColors(
    val important: Color,
    val tip: Color,
    val feedback: Color
)

val LocalFeedbackColors = staticCompositionLocalOf {
    FeedbackColors(
        important = Color.Red,
        tip = Color.Blue,
        feedback = Color.Green
    )
}

// Dark Mode: Background is Black (RichBlack), Text is White with slight yellow (WarmWhite)
private val DarkColorScheme = darkColorScheme(
    primary = LightPurple,
    onPrimary = RichBlack,
    primaryContainer = MildPurple,
    onPrimaryContainer = WarmWhite,
    secondary = LightPurple,
    onSecondary = RichBlack,
    tertiary = LightPurple,
    onTertiary = RichBlack,
    background = RichBlack,
    onBackground = WarmWhite,
    surface = RichBlack,
    onSurface = WarmWhite,
    surfaceVariant = RichBlack,
    onSurfaceVariant = WarmWhite,
    outline = LightPurple,
    outlineVariant = LightPurple,
    error = red,
    onError = red,
)

// Light Mode: Background is White with slight yellow (WarmWhite), Text is Black (RichBlack)
private val LightColorScheme = lightColorScheme(
    primary = MildPurple,
    onPrimary = WarmWhite,
    primaryContainer = WarmWhite,
    onPrimaryContainer = MildPurple,
    secondary = LightPurple,
    onSecondary = WarmWhite,
    tertiary = MildPurple,
    onTertiary = WarmWhite,
    background = WarmWhite,
    onBackground = RichBlack,
    surface = WarmWhite,
    onSurface = RichBlack,
    surfaceVariant = WarmWhite,
    onSurfaceVariant = RichBlack,
    outline = MildPurple,
    outlineVariant = MildPurple,
    error = red,
    onError = red
)

@Composable
fun GlobalMaterialTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    content: @Composable () -> Unit
) {
    val colorScheme = if (darkTheme) DarkColorScheme else LightColorScheme
    val feedbackColors = if (darkTheme) {
        FeedbackColors(
            important = ImportantRedDark,
            tip = TipBlueDark,
            feedback = FeedbackGreenDark
        )
    } else {
        FeedbackColors(
            important = ImportantRed,
            tip = TipBlue,
            feedback = FeedbackGreen
        )
    }

    CompositionLocalProvider(LocalFeedbackColors provides feedbackColors) {
        MaterialTheme(
            colorScheme = colorScheme,
            typography = MeliorSonusTypography,
            content = content
        )
    }
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

    val feedbackColors: FeedbackColors
        @Composable
        @ReadOnlyComposable
        get() = LocalFeedbackColors.current
}
