package com.example.meliorsonus.model

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class SheetSearchResult(
    val mxl: String,
    val pdf: String,
    val title: String,
    @SerialName("song_length.bars") val songLengthBars: Int,
    val genres: String,
    val instruments: List<String>,
    val composer: String
)

@Serializable
data class SheetSearchResponse(
    @SerialName("results") val results: Map<String, SheetSearchResult>
)
