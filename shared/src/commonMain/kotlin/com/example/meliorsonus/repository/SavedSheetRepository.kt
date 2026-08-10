package com.example.meliorsonus.repository

import com.example.meliorsonus.db.MeliorSonusDatabase
import com.example.meliorsonus.SavedSheet
import com.example.meliorsonus.util.appFilesDir
import com.example.meliorsonus.util.systemFileSystem
import com.example.meliorsonus.util.ioDispatcher
import kotlinx.coroutines.withContext
import kotlinx.serialization.ExperimentalSerializationApi
import kotlinx.serialization.encodeToString
import okio.Path
import okio.Path.Companion.toPath
import okio.buffer
import okio.use
import kotlinx.serialization.json.Json
import kotlinx.serialization.decodeFromString

interface SavedSheetRepository {
    suspend fun saveSheet(
        mxl: String,
        pdf: String,
        title: String,
        composer: String,
        songLengthBars: Long,
        genres: String,
        instruments: String,
    )
    suspend fun getAllSavedSheets(): List<SavedSheet>
    suspend fun deleteSheet(mxl: String)
    suspend fun checkExistance(mxl: String): Boolean
}

class SavedSheetRepositoryImpl(
    private val database: MeliorSonusDatabase
) : SavedSheetRepository {
    private val queries = database.savedSheetQueries

    override suspend fun saveSheet(
        mxl: String,
        pdf: String,
        title: String,
        composer: String,
        songLengthBars: Long,
        genres: String,
        instruments: String,
    ) {
        withContext(ioDispatcher) {
            queries.insertSavedSheet(
                mxl = mxl,
                pdf_icon = pdf,
                title = title,
                composer = composer,
                song_length_bars = songLengthBars,
                genres = genres,
                instruments = instruments,
                saved_at = com.example.meliorsonus.db.getCurrentEpochMillis()
            )
        }
    }

    override suspend fun getAllSavedSheets(): List<SavedSheet> = withContext(ioDispatcher) {
        queries.getAllSavedSheets().executeAsList()
    }

    override suspend fun deleteSheet(mxl: String) {
        withContext(ioDispatcher) {
            val normalizedMxl = mxl.replace("\\", "/")
            val baseDir = appFilesDir.toPath()
            
            val mxlDir = baseDir / "mxl" / mxl
            
            try {
                if (systemFileSystem.exists(mxlDir)) {
                    systemFileSystem.deleteRecursively(mxlDir)
                }
            } catch (_: Exception) {
                // Ignore deletion errors for specific files
            }
            queries.deleteSavedSheet(normalizedMxl)
        }
    }

    override suspend fun checkExistance(mxl: String): Boolean = withContext(ioDispatcher) {
        queries.checkExists(mxl).executeAsOne()
    }
}
