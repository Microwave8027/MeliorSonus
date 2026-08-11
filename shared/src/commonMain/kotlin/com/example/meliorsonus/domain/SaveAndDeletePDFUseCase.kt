package com.example.meliorsonus.domain

import com.example.meliorsonus.repository.SheetSearchRepository
import com.example.meliorsonus.util.appFilesDir
import com.example.meliorsonus.util.ioDispatcher
import com.example.meliorsonus.util.systemFileSystem
import kotlinx.coroutines.withContext
import okio.Path.Companion.toPath

class SaveAndDeletePDFUseCase(
    private val sheetSearchRepository: SheetSearchRepository
) {
    suspend operator fun invoke(path: String) = withContext(ioDispatcher) {
        sheetSearchRepository.deletePdf(path)
    }
}