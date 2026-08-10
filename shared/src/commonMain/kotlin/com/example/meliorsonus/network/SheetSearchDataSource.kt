package com.example.meliorsonus.network

import com.example.meliorsonus.model.SheetSearchResult
import com.example.meliorsonus.model.SheetSearchResponse
import io.ktor.client.HttpClient
import io.ktor.client.call.*
import io.ktor.client.request.*
import io.ktor.client.statement.*
import io.ktor.utils.io.ByteReadChannel
import kotlinx.coroutines.withContext
import com.example.meliorsonus.util.ioDispatcher

class SheetSearchDataSource(private val client: HttpClient) {

    // Returns sorted list of results (index 0..9) from /search endpoint
    suspend fun searchSheets(query: String, pages: Int, instrument: String): List<SheetSearchResult> = withContext(ioDispatcher) {
        val response: SheetSearchResponse = client.get("$baseUrl/search") {
            parameter("query", query)
            parameter("index", pages)
            parameter("instrument", instrument)
        }.body()
        response.results.values.toList()
    }

    // Streams PDF bytes from /pdf endpoint
    suspend fun fetchPdf(pdfPath: String): ByteReadChannel {
        return client.get("$baseUrl/pdf") {
            parameter("path", pdfPath)
        }.bodyAsChannel()
    }

    // Streams MXL bytes from /mxl endpoint
    suspend fun fetchMXL(mxlPath: String): ByteReadChannel {
        return client.get("$baseUrl/mxl") {
            parameter("path", mxlPath)
        }.bodyAsChannel()
    }
}
