package com.example.meliorsonus.db

actual fun getCurrentEpochMillis(): Long = kotlin.js.Date.now().toLong()
