package com.example.meliorsonus.domain

import com.example.meliorsonus.model.SheetSearchResult
import com.example.meliorsonus.repository.SavedSheetRepository
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

class SaveSheetUseCase(
    private val savedSheetRepository: SavedSheetRepository,
) {
    suspend operator fun invoke(result: SheetSearchResult): String = withContext(Dispatchers.Default) {

        val normalizedMxlPath = result.mxl.replace("\\", "/")

        println("SaveSheetUseCase: Saving path: ${normalizedMxlPath.take(30)}")

        savedSheetRepository.saveSheet(
            mxl = normalizedMxlPath,
            title = result.title,
            artistName = result.artistName,
            composerName = result.composerName,
            publisher = result.publisher,
            songLengthBars = result.songLengthBars.toLong(),
            genres = result.genres,
            instruments = result.instruments.joinToString(", "),
        )
        normalizedMxlPath
    }
}
