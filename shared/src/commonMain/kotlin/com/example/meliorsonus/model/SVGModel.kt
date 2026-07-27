package com.example.meliorsonus.model

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class SVGModel(
    @SerialName("total_pages")
    val totalPages: Int,
    val pages: List<SVGPage>
)


@Serializable
data class SVGPage(
    @SerialName("page_index")
    val pageIndex: Int,
    @SerialName("svg_string")
    val svgString: String,
    @SerialName("original_height")
    val originalHeight: Float,
    @SerialName("original_width")
    val originalWidth: Float,
    val measures: List<MeasureLayout>,
    val notes: List<NoteLayout>
)

data class SVG(
    val device: String = "tablet",
    val pathToSVG: String,
    val totalPages: Int,
    val metaData: List<ScoreMetadata>,
    val isError: Boolean = false
)

data class ScoreMetadata(
    val originalWidth: Float = 0f,
    val originalHeight: Float = 0f,
    val measures: List<MeasureLayout> = emptyList(),
    val notes: List<NoteLayout> = emptyList()
)
@Serializable
data class svgContent(
    val pageIndex: Int,
    val svgString: String
)

@Serializable
data class MeasureLayout(val id: String, val x: Float, val y: Float, val w: Float, val h: Float)

@Serializable
data class NoteLayout(val id: String, val x: Float, val y: Float, val w: Float, val h: Float, val pitch: String, val duration: String, val dots: Int)