package com.example.meliorsonus.db

import app.cash.sqldelight.db.SqlDriver
import app.cash.sqldelight.driver.native.NativeSqliteDriver

actual class DatabaseDriverFactory {
    actual fun create(): SqlDriver {
        return NativeSqliteDriver(MeliorSonusDatabase.Schema, "melior_sonus.db")
    }
}
