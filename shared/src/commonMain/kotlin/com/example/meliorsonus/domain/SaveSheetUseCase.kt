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
        val normalizedPDFPath = result.pdf.replace("\\", "/").substringAfterLast("/")

        println("SaveSheetUseCase: Saving path: ${normalizedMxlPath.take(30)}")
        if (!savedSheetRepository.checkExistance(mxl = result.mxl)) {
            savedSheetRepository.saveSheet(
                mxl = normalizedMxlPath,
                pdf = normalizedPDFPath,
                title = result.title,
                composer = result.composer,
                songLengthBars = result.songLengthBars.toLong(),
                genres = result.genres,
                instruments = result.instruments.joinToString(", "),
            )
        }
        normalizedMxlPath
    }
}
