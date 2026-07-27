package com.example.meliorsonus.domain

import com.example.meliorsonus.model.SheetSearchResult
import com.example.meliorsonus.repository.SavedSheetRepository
import com.example.meliorsonus.repository.SheetSearchRepository
import com.example.meliorsonus.util.SizeOfScreen
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

class SaveSheetUseCase(
    private val sheetSearchRepository: SheetSearchRepository,
    private val savedSheetRepository: SavedSheetRepository,
    private val sizeOfScreen: SizeOfScreen
) {
    suspend operator fun invoke(result: SheetSearchResult, zoom: Float = 1.0f): String = withContext(Dispatchers.Default) {
        val (widthDp, heightDp, density) = sizeOfScreen.getSize()
        
        // Calculations in DP
        val longerDp = maxOf(widthDp, heightDp)
        val shorterDp = minOf(widthDp, heightDp)
        
        val device: String
        val finalHeightPx = ((longerDp - 64) / 2 * density).toInt()
        val finalWidthPx = (shorterDp * density).toInt()
        
        if (shorterDp <= 600) {
            device = "phone"
        } else {
            device = "tablet"
        }

        val normalizedMxlPath = result.mxl.replace("\\", "/")

        println("SaveSheetUseCase: Saving path: ${normalizedMxlPath.take(30)}")

        sheetSearchRepository.fetchSVG(
            mxlPath = normalizedMxlPath,
            width = finalWidthPx,
            height = finalHeightPx,
            zoom = (zoom * 100).toInt(),
            device = device
        )

        savedSheetRepository.saveSheet(
            mxl = normalizedMxlPath,
            title = result.title,
            artistName = result.artistName,
            composerName = result.composerName,
            publisher = result.publisher,
            songLengthBars = result.songLengthBars.toLong(),
            genres = result.genres,
            instruments = result.instruments.joinToString(", "),
            tablet = if (device == "tablet") 1 else 0,
            phone = if(device == "phone") 1 else 0
        )
        normalizedMxlPath
    }
}
