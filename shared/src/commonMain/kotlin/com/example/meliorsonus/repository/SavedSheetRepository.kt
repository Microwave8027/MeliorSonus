package com.example.meliorsonus.repository

import com.example.meliorsonus.db.MeliorSonusDatabase
import com.example.meliorsonus.SavedSheet
import com.example.meliorsonus.model.MeasureLayout
import com.example.meliorsonus.model.NoteLayout
import com.example.meliorsonus.model.SVG
import com.example.meliorsonus.model.SVGModel
import com.example.meliorsonus.model.ScoreMetadata
import com.example.meliorsonus.model.svgContent
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
        title: String,
        artistName: String,
        composerName: String,
        publisher: String,
        songLengthBars: Long,
        genres: String,
        instruments: String,
        tablet: Int,
        phone: Int
    )
    suspend fun getAllSavedSheets(): List<SavedSheet>
    suspend fun deleteSheet(mxl: String)
    suspend fun getSVG(mxl: String, device: String): SVG
    suspend fun checkDeviceExistence(mxl: String, device: String): Boolean
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
        instruments: String,
        tablet: Int,
        phone: Int
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
                tablet = tablet.toLong(),
                phone = phone.toLong(),
                saved_at = com.example.meliorsonus.db.getCurrentEpochMillis()
            )
        }
    }

    override suspend fun checkDeviceExistence(mxl: String, device: String): Boolean = withContext(ioDispatcher){
        val fileName = mxl.replace("\\", "/").substringAfterLast("/")
        systemFileSystem.exists(appFilesDir.toPath() / "raw" / device / fileName)
    }

    @OptIn(ExperimentalSerializationApi::class)
    override suspend fun getSVG(mxl: String, device: String): SVG = withContext(ioDispatcher){
        val fileName = mxl.replace("\\", "/").substringAfterLast("/")
        val rawPath: Path = appFilesDir.toPath() / "raw" / device / fileName

        val json = Json { ignoreUnknownKeys = true }
        val svgOutputPath = appFilesDir.toPath() / "svg" / device / fileName
        svgOutputPath.parent?.let { systemFileSystem.createDirectories(it) }

        val rawContent = systemFileSystem.source(rawPath).buffer().use { it.readUtf8() }
        // rawContent will contain a json

        try {
            val response: SVGModel = json.decodeFromString(rawContent)
            val svgContent: List<svgContent> = response.pages.map { page ->
                svgContent(
                    pageIndex = page.pageIndex,
                    svgString = page.svgString
                )
            }

            systemFileSystem.sink(svgOutputPath).buffer().use { it.writeUtf8(Json.encodeToString(svgContent)) }
            val metadata: List<ScoreMetadata> = response.pages.map { page ->
                ScoreMetadata(
                    originalWidth = page.originalWidth,
                    originalHeight = page.originalHeight,
                    measures = page.measures,
                    notes = page.notes
                )
            }

            SVG(
                device = device,
                pathToSVG = svgOutputPath.toString(),
                totalPages = response.totalPages,
                metaData = metadata
            )
        } catch (e: Exception) {
            svgOutputPath.parent?.let { systemFileSystem.createDirectories(it) }
            systemFileSystem.sink(svgOutputPath).buffer().use { it.writeUtf8(rawContent) }
            println("Error processing SVG for $mxl: ${e.message}")
            SVG(
                device = device,
                pathToSVG = svgOutputPath.toString(),
                totalPages = 1,
                metaData = emptyList(),
                isError = true
            )
        }
    }

    override suspend fun getAllSavedSheets(): List<SavedSheet> = withContext(ioDispatcher) {
        queries.getAllSavedSheets().executeAsList()
    }

    override suspend fun deleteSheet(mxl: String) {
        withContext(ioDispatcher) {
            val normalizedMxl = mxl.replace("\\", "/")
            val fileName = normalizedMxl.substringAfterLast("/")
            val baseDir = appFilesDir.toPath()
            
            val filesToDelete = listOf(
                baseDir / "raw" / "tablet" / fileName,
                baseDir / "raw" / "phone" / fileName,
                baseDir / "svg" / "tablet" / fileName,
                baseDir / "svg" / "phone" / fileName
            )
            
            for (file in filesToDelete) {
                try {
                    if (systemFileSystem.exists(file)) {
                        systemFileSystem.delete(file)
                    }
                } catch (_: Exception) {
                    // Ignore deletion errors for specific files
                }
            }
            queries.deleteSavedSheet(normalizedMxl)
        }
    }
}
