package com.example.meliorsonus.network

import com.example.meliorsonus.model.SheetSearchResult
import com.example.meliorsonus.model.SheetSearchResponse
import io.ktor.client.call.*
import io.ktor.client.request.*
import io.ktor.client.statement.*
import io.ktor.utils.io.ByteReadChannel
import io.ktor.utils.io.readAvailable
import okio.Path.Companion.toPath
import okio.buffer
import okio.use
import com.example.meliorsonus.util.systemFileSystem
import com.example.meliorsonus.util.ioDispatcher
import com.example.meliorsonus.util.unzipMusicXml
import io.ktor.client.HttpClient
import kotlinx.coroutines.withContext

class SheetSearchDataSource(private val client: HttpClient) {

    // Returns sorted list of results (index 0..9) from /search endpoint
    suspend fun searchSheets(query: String): List<SheetSearchResult> = withContext(ioDispatcher) {
        val response: SheetSearchResponse = client.get("$baseUrl/search") {
            parameter("query", query)
        }.body()
        response.results.values.toList()
    }

    // Streams PDF bytes from /pdf endpoint and saves to a temporary file, returns path
    suspend fun fetchPdf(pdfPath: String): String {
        val response: ByteReadChannel = client.get("$baseUrl/pdf") {
            parameter("path", pdfPath)
        }.bodyAsChannel()
        val fileName = "temp_" + pdfPath.hashCode().coerceAtLeast(0) + ".pdf"
        val path = com.example.meliorsonus.util.tempDir.toPath() / fileName
        
        withContext(ioDispatcher) {
            systemFileSystem.sink(path).buffer().use { sink ->
                val buffer = ByteArray(8 * 1024)
                while (!response.isClosedForRead) {
                    val bytesRead = response.readAvailable(buffer, 0, buffer.size)
                    if (bytesRead > 0) {
                        sink.write(buffer, 0, bytesRead)
                    }
                }
            }
        }
        return path.toString()
    }

    suspend fun deletePdf(tempPath: String) = withContext(ioDispatcher) {
        try {
            systemFileSystem.delete(tempPath.toPath())
        } catch (e: Exception) {
            // Ignore deletion errors for temporary files
        }
    }

    suspend fun fetchMXL(mxlPath: String): String {
        val response: ByteReadChannel = client.get("$baseUrl/mxl") {
            parameter("path", mxlPath)
        }.bodyAsChannel()

        val originalFileName = mxlPath.substringAfterLast("/").substringAfterLast("\\")
        val xmlFileName = originalFileName.substringBeforeLast(".") + ".xml"

        val appDir = com.example.meliorsonus.util.appFilesDir.toPath()
        val tempMxlPath = appDir / "$originalFileName.mxl"
        val xmlPath = appDir / xmlFileName

        withContext(ioDispatcher) {
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
            
            // Extract the .xml file from the .mxl (zip) archive
            unzipMusicXml(tempMxlPath, xmlPath)
            
            // Clean up the temporary .mxl file
            try {
                systemFileSystem.delete(tempMxlPath)
            } catch (e: Exception) {
                // Ignore cleanup errors
            }
        }
        return xmlPath.toString()
    }


}
