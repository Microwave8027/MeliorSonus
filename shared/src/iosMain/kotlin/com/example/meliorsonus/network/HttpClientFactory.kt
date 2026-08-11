package com.example.meliorsonus.network

import io.ktor.client.*
import io.ktor.client.engine.darwin.Darwin
import io.ktor.client.engine.mock.MockEngine
import io.ktor.client.engine.mock.respondOk
import io.ktor.client.plugins.contentnegotiation.*
import io.ktor.serialization.kotlinx.json.*
import kotlinx.serialization.json.Json

actual val createHttpClient: HttpClient = HttpClient(Darwin) {
    install(ContentNegotiation) {
        json(Json {
            ignoreUnknownKeys = true
            isLenient = true
        })
    }
}


actual val baseUrl: String = "http://127.0.0.1:8000"
