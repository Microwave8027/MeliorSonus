package com.example.meliorsonus.repository

import com.example.meliorsonus.db.MeliorSonusDatabase
import com.example.meliorsonus.SavedSheet
import com.example.meliorsonus.util.appFilesDir
import com.example.meliorsonus.util.systemFileSystem
import com.example.meliorsonus.util.ioDispatcher
import kotlinx.coroutines.withContext
import okio.Path
import okio.Path.Companion.toPath
import okio.buffer
import okio.use

interface SavedSheetRepository {
    suspend fun saveSheet(
        mxl: String,
        title: String,
        artistName: String,
        composerName: String,
        publisher: String,
        songLengthBars: Long,
        genres: String,
        instruments: String
    )
    suspend fun getAllSavedSheets(): List<SavedSheet>
    suspend fun deleteSheet(mxl: String)
    suspend fun getMXL(path: String): String
}

class SavedSheetRepositoryImpl(
    private val database: MeliorSonusDatabase
) : SavedSheetRepository {
    private val queries = database.savedSheetQueries

    override suspend fun saveSheet(
        mxl: String,
        title: String,
        artistName: String,
        composerName: String,
        publisher: String,
        songLengthBars: Long,
        genres: String,
        instruments: String
    ) {
        withContext(ioDispatcher) {
            queries.insertSavedSheet(
                mxl = mxl,
                title = title,
                artist_name = artistName,
                composer_name = composerName,
                publisher = publisher,
                song_length_bars = songLengthBars,
                genres = genres,
                instruments = instruments,
                saved_at = com.example.meliorsonus.db.getCurrentEpochMillis()
            )
        }
    }

    override suspend fun getMXL(path: String): String {
        return withContext(ioDispatcher) {
            val conversion: Path = appFilesDir.toPath() / path
            systemFileSystem.source(conversion).buffer().use { source ->
                source.readUtf8()
            }
        }
    }

    override suspend fun getAllSavedSheets(): List<SavedSheet> = withContext(ioDispatcher) {
        queries.getAllSavedSheets().executeAsList()
    }

    override suspend fun deleteSheet(mxl: String) {
        withContext(ioDispatcher) {
            try {
                val path = mxl.toPath()
                val fileToDelete = if (path.isAbsolute) path else appFilesDir.toPath() / path
                if (systemFileSystem.exists(fileToDelete)) {
                    systemFileSystem.delete(fileToDelete)
                }
            } catch (_: Exception) {
                // Ignore deletion errors, but proceed to delete from DB
            }
            queries.deleteSavedSheet(mxl)
        }
    }
}
