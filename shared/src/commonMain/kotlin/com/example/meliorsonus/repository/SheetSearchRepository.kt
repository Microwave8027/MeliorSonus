package com.example.meliorsonus.repository

import com.example.meliorsonus.model.SheetSearchResult
import com.example.meliorsonus.network.SheetSearchDataSource

interface SheetSearchRepository {
    suspend fun searchSheets(query: String): List<SheetSearchResult>
    suspend fun fetchPdf(pdfPath: String): String
    suspend fun deletePdf(tempPath: String)

    suspend fun fetchMXL(mxlPath: String): String
}

class SheetSearchRepositoryImpl(
    private val dataSource: SheetSearchDataSource
) : SheetSearchRepository {
    override suspend fun searchSheets(query: String): List<SheetSearchResult> =
        dataSource.searchSheets(query)

    override suspend fun fetchPdf(pdfPath: String): String =
        dataSource.fetchPdf(pdfPath)

    override suspend fun deletePdf(tempPath: String) {
        dataSource.deletePdf(tempPath)
    }

    override suspend fun fetchMXL(mxlPath: String): String =
        dataSource.fetchMXL(mxlPath)
}
