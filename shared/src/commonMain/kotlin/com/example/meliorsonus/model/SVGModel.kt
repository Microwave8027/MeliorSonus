package com.example.meliorsonus.model

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class SVGResponse(
    @SerialName("svg_string")
    val svgString: String,
    @SerialName("original_height")
    val originalHeight: Float,
    @SerialName("original_width")
    val originalWidth: Float,
    val measure: Float
)


@Serializable
data class MeasureLayout(val index: Int, val x: Float, val y: Float, val width: Float, val height: Float)

@Serializable
data class NoteLayout(val pitch: String, val x: Float, val y: Float)