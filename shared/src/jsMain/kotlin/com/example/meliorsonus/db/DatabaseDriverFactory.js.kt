package com.example.meliorsonus.db

import app.cash.sqldelight.db.SqlDriver

actual class DatabaseDriverFactory {
    actual fun create(): SqlDriver =
        throw UnsupportedOperationException("SQLDelight not supported on JS target")
}
