package com.example.meliorsonus.domain

import com.example.meliorsonus.model.SheetSearchResult
import com.example.meliorsonus.repository.SavedSheetRepository
import com.example.meliorsonus.repository.SheetSearchRepository

class SaveSheetUseCase(
    private val sheetSearchRepository: SheetSearchRepository,
    private val savedSheetRepository: SavedSheetRepository
) {
    suspend operator fun invoke(result: SheetSearchResult): String {
        val savedPath = sheetSearchRepository.fetchMXL(result.mxl)

        savedSheetRepository.saveSheet(
            mxl = savedPath,
            title = result.title,
            artistName = result.artistName,
            composerName = result.composerName,
            publisher = result.publisher,
            songLengthBars = result.songLengthBars.toLong(),
            genres = result.genres,
            instruments = result.instruments.joinToString(", ")
        )
        return savedPath
    }
}
