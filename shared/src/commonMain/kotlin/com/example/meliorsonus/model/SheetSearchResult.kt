package com.example.meliorsonus.model

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class SheetSearchResult(
    val mxl: String,
    val pdf: String,
    val title: String,
    @SerialName("artist_name") val artistName: String,
    @SerialName("composer_name") val composerName: String,
    val publisher: String,
    @SerialName("song_length.bars") val songLengthBars: Int = 0,
    val genres: String = "",
    val instruments: List<String> = emptyList()
)

@Serializable
data class SheetSearchResponse(
    val results: Map<String, SheetSearchResult>
)
