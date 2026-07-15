package com.gitswamp.mobile.core.designsystem

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

@Composable
fun GitSwampBrand(
    modifier: Modifier = Modifier,
    compact: Boolean = false,
) {
    Row(modifier = modifier, verticalAlignment = Alignment.CenterVertically) {
        Canvas(Modifier.size(if (compact) 26.dp else 34.dp)) {
            val stroke = size.minDimension * 0.14f
            val path = Path().apply {
                moveTo(size.width * 0.18f, size.height * 0.56f)
                cubicTo(
                    size.width * 0.2f,
                    size.height * 0.28f,
                    size.width * 0.48f,
                    size.height * 0.18f,
                    size.width * 0.72f,
                    size.height * 0.35f,
                )
                lineTo(size.width * 0.88f, size.height * 0.31f)
                lineTo(size.width * 0.78f, size.height * 0.5f)
                cubicTo(
                    size.width * 0.67f,
                    size.height * 0.76f,
                    size.width * 0.3f,
                    size.height * 0.82f,
                    size.width * 0.18f,
                    size.height * 0.56f,
                )
            }
            drawPath(path, SwampColors.Primary, style = Stroke(width = stroke, cap = StrokeCap.Round))
            drawCircle(SwampColors.Green, size.minDimension * 0.065f, Offset(size.width * 0.7f, size.height * 0.38f))
        }
        Text(
            text = "GitSwamp",
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.Bold,
        )
        if (!compact) {
            Text(
                text = "MOBILE",
                modifier = Modifier.padding(start = 8.dp),
                color = SwampColors.Primary,
                fontSize = 9.sp,
                fontWeight = FontWeight.Bold,
            )
        }
    }
}

@Composable
fun SwampPrimaryButton(
    text: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    leading: (@Composable () -> Unit)? = null,
) {
    Button(
        onClick = onClick,
        modifier = modifier.height(44.dp),
        enabled = enabled,
        shape = RoundedCornerShape(6.dp),
        colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.primary),
    ) {
        leading?.invoke()
        Text(text, modifier = if (leading != null) Modifier.padding(start = 7.dp) else Modifier)
    }
}

@Composable
fun SwampSecondaryButton(
    text: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    leading: (@Composable () -> Unit)? = null,
) {
    Button(
        onClick = onClick,
        modifier = modifier.height(44.dp),
        enabled = enabled,
        shape = RoundedCornerShape(6.dp),
        colors = ButtonDefaults.buttonColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant,
            contentColor = MaterialTheme.colorScheme.onSurface,
        ),
        border = androidx.compose.foundation.BorderStroke(1.dp, MaterialTheme.colorScheme.outline),
    ) {
        leading?.invoke()
        Text(text, modifier = if (leading != null) Modifier.padding(start = 7.dp) else Modifier)
    }
}

@Composable
fun StatusPill(
    text: String,
    color: Color,
    modifier: Modifier = Modifier,
) {
    Surface(
        modifier = modifier,
        shape = RoundedCornerShape(5.dp),
        color = color.copy(alpha = 0.12f),
        border = androidx.compose.foundation.BorderStroke(1.dp, color.copy(alpha = 0.35f)),
    ) {
        Text(
            text = text,
            modifier = Modifier.padding(horizontal = 7.dp, vertical = 3.dp),
            color = color,
            style = MaterialTheme.typography.labelSmall,
        )
    }
}

@Composable
fun LoadingState(label: String, modifier: Modifier = Modifier) {
    Column(
        modifier = modifier.fillMaxWidth().padding(32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        CircularProgressIndicator(Modifier.size(28.dp), strokeWidth = 2.dp)
        Spacer(Modifier.height(14.dp))
        Text(label, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

@Composable
fun InlineNotice(
    title: String,
    detail: String,
    color: Color,
    modifier: Modifier = Modifier,
) {
    Column(
        modifier = modifier
            .fillMaxWidth()
            .background(color.copy(alpha = 0.07f), RoundedCornerShape(6.dp))
            .border(1.dp, color.copy(alpha = 0.3f), RoundedCornerShape(6.dp))
            .padding(12.dp),
    ) {
        Text(title, color = color, fontWeight = FontWeight.SemiBold, style = MaterialTheme.typography.bodyMedium)
        Spacer(Modifier.height(3.dp))
        Text(detail, color = MaterialTheme.colorScheme.onSurfaceVariant, style = MaterialTheme.typography.bodySmall)
    }
}
