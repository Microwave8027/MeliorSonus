package com.example.meliorsonus.network

import io.ktor.client.*

/**
 * Provides a platform-specific [HttpClient] engine.
 * Each platform supplies its own engine (CIO for JVM/Android/iOS, JS for web targets).
 */
expect val createHttpClient: HttpClient
expect val baseUrl: String
