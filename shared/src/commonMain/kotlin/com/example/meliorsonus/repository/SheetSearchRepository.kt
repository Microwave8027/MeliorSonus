package com.example.meliorsonus.repository

import com.example.meliorsonus.model.SheetSearchResult
import com.example.meliorsonus.network.SheetSearchDataSource
import com.example.meliorsonus.util.appFilesDir
import com.example.meliorsonus.util.systemFileSystem
import com.example.meliorsonus.util.tempDir
import com.example.meliorsonus.util.unzipMusicXml
import com.example.meliorsonus.util.trimMusicXmlTo10Measures
import com.example.meliorsonus.util.ioDispatcher
import io.ktor.utils.io.readAvailable
import kotlinx.coroutines.withContext
import okio.Path.Companion.toPath
import okio.buffer
import okio.use

interface SheetSearchRepository {
    suspend fun searchSheets(query: String): List<SheetSearchResult>
    suspend fun fetchPdf(pdfPath: String): String
    suspend fun deletePdf(tempPath: String)
    suspend fun deleteMXL(tempPath: String)
    suspend fun fetchMXL(mxlPath: String): Pair<String, String> // Path, XML Content

    suspend fun fetchSVG(mxlPath: String, width: Int, height: Int, zoom: Int = 100, device: String = "tablet"): String
}

class SheetSearchRepositoryImpl(
    private val dataSource: SheetSearchDataSource
) : SheetSearchRepository {
    override suspend fun searchSheets(query: String): List<SheetSearchResult> =
        dataSource.searchSheets(query)

    override suspend fun fetchPdf(pdfPath: String): String = withContext(ioDispatcher) {
        val response = dataSource.fetchPdf(pdfPath)
        val fileName = "temp_" + pdfPath.hashCode().coerceAtLeast(0) + ".pdf"
        val path = tempDir.toPath() / fileName
        
        systemFileSystem.sink(path).buffer().use { sink ->
            val buffer = ByteArray(8 * 1024)
            while (!response.isClosedForRead) {
                val bytesRead = response.readAvailable(buffer, 0, buffer.size)
                if (bytesRead > 0) {
                    sink.write(buffer, 0, bytesRead)
                }
            }
        }
        path.toString()
    }

    override suspend fun deletePdf(tempPath: String) {
        withContext(ioDispatcher) {
            try {
                systemFileSystem.delete(tempPath.toPath())
            } catch (_: Exception) {
                // Ignore deletion errors for temporary files
            }
        }
    }

    override suspend fun fetchMXL(mxlPath: String): Pair<String, String> = withContext(ioDispatcher) {
        val normalizedMxlPath = mxlPath.replace("\\", "/")
        val response = dataSource.fetchMXL(normalizedMxlPath)

        val originalFileName = normalizedMxlPath.substringAfterLast("/")
        val xmlFileName = originalFileName.substringBeforeLast(".") + ".xml"

        val appDir = appFilesDir.toPath()
        val tempMxlPath = appDir / "$originalFileName.mxl"
        val xmlPath = appDir / xmlFileName

        systemFileSystem.sink(tempMxlPath).buffer().use { sink ->
            val buffer = ByteArray(8 * 1024)
            while (!response.isClosedForRead) {
                val bytesRead = response.readAvailable(buffer, 0, buffer.size)
                if (bytesRead < 0) break
                if (bytesRead > 0) {
                    sink.write(buffer, 0, bytesRead)
                }
            }
        }
        
        unzipMusicXml(tempMxlPath, xmlPath)
        
        try {
            systemFileSystem.delete(tempMxlPath)
        } catch (_: Exception) {}

        var trimmedXml = ""
        try {
            val rawXml = systemFileSystem.source(xmlPath).buffer().use { it.readUtf8() }
            trimmedXml = trimMusicXmlTo10Measures(rawXml)
            systemFileSystem.sink(xmlPath).buffer().use { sink ->
                sink.writeUtf8(trimmedXml)
            }
        } catch (_: Exception) {}

        xmlPath.toString() to trimmedXml
    }

    override suspend fun deleteMXL(tempPath: String) {
        withContext(ioDispatcher) {
            try {
                if (tempPath.isNotBlank()) {
                    val path = tempPath.toPath()
                    if (systemFileSystem.exists(path)) {
                        systemFileSystem.delete(path)
                    }
                }
            } catch (_: Exception) {
                // Ignore deletion errors
            }
        }
    }

    override suspend fun fetchSVG(mxlPath: String, width: Int, height: Int, zoom: Int, device: String): String = withContext(ioDispatcher) {
        val normalizedMxlPath = mxlPath.replace("\\", "/")
        val response = dataSource.fetchSVG(normalizedMxlPath, height, width, zoom)
        val fileName = normalizedMxlPath.substringAfterLast("/")
        val filePath = appFilesDir.toPath() / "raw" / device / fileName

        filePath.parent?.let { systemFileSystem.createDirectories(it) }
        systemFileSystem.sink(filePath).buffer().use { sink ->
            sink.writeUtf8(response)
        }
        normalizedMxlPath
    }
}
