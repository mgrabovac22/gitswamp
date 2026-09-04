package com.gitswamp.mobile.core.designsystem

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp

object SwampColors {
    val Deep = Color(0xFF090B10)
    val Background = Color(0xFF0D1017)
    val Card = Color(0xFF151921)
    val Header = Color(0xFF1C2130)
    val Border = Color(0x268B5CF6)
    val Primary = Color(0xFF8B5CF6)
    val PrimaryDark = Color(0xFF6D28D9)
    val Foreground = Color(0xFFE2E8F0)
    val Muted = Color(0xFF64748B)
    val Cyan = Color(0xFF38BDF8)
    val Green = Color(0xFF34D399)
    val Amber = Color(0xFFF59E0B)
    val Red = Color(0xFFEF4444)
    val DiffAdded = Color(0xFFAFF5B4)
    val DiffAddedBackground = Color(0x7F1A4D2E)
    val DiffDeleted = Color(0xFFFFA198)
    val DiffDeletedBackground = Color(0x7F4D1A1A)
}

private val DarkColors = darkColorScheme(
    primary = SwampColors.Primary,
    onPrimary = Color.White,
    primaryContainer = Color(0x332F855A),
    onPrimaryContainer = SwampColors.Green,
    secondary = SwampColors.Header,
    onSecondary = SwampColors.Foreground,
    background = SwampColors.Background,
    onBackground = SwampColors.Foreground,
    surface = SwampColors.Card,
    onSurface = SwampColors.Foreground,
    surfaceVariant = SwampColors.Header,
    onSurfaceVariant = Color(0xFF94A3B8),
    outline = Color(0x3D8B5CF6),
    error = SwampColors.Red,
)

private val LightColors = lightColorScheme(
    primary = Color(0xFF7C3AED),
    onPrimary = Color.White,
    secondary = Color(0xFFE2E8F0),
    onSecondary = Color(0xFF1E293B),
    background = Color(0xFFF0F3F7),
    onBackground = Color(0xFF1A202C),
    surface = Color(0xFFF8FAFC),
    onSurface = Color(0xFF1A202C),
    surfaceVariant = Color(0xFFE2E8F0),
    onSurfaceVariant = Color(0xFF475569),
    outline = Color(0x33475569),
    error = Color(0xFFDC2626),
)

private val GitSwampTypography
    @Composable get() = MaterialTheme.typography.copy(
        titleLarge = TextStyle(fontSize = 20.sp, fontWeight = FontWeight.Bold),
        titleMedium = TextStyle(fontSize = 15.sp, fontWeight = FontWeight.SemiBold),
        titleSmall = TextStyle(fontSize = 13.sp, fontWeight = FontWeight.SemiBold),
        bodyLarge = TextStyle(fontSize = 15.sp, fontWeight = FontWeight.Normal),
        bodyMedium = TextStyle(fontSize = 13.sp, fontWeight = FontWeight.Normal),
        bodySmall = TextStyle(fontSize = 11.sp, fontWeight = FontWeight.Normal),
        labelLarge = TextStyle(fontSize = 13.sp, fontWeight = FontWeight.SemiBold),
        labelMedium = TextStyle(fontSize = 11.sp, fontWeight = FontWeight.Medium),
        labelSmall = TextStyle(fontSize = 10.sp, fontWeight = FontWeight.Medium),
    )

@Composable
fun GitSwampTheme(
    useSystemTheme: Boolean,
    content: @Composable () -> Unit,
) {
    val dark = !useSystemTheme || isSystemInDarkTheme()
    MaterialTheme(
        colorScheme = if (dark) DarkColors else LightColors,
        typography = GitSwampTypography,
        content = content,
    )
}

val MonoFontFamily = FontFamily.Monospace

