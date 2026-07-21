package com.example.meliorsonus

import app.cash.sqldelight.db.SqlDriver
import app.cash.sqldelight.driver.native.NativeSqliteDriver
import com.example.meliorsonus.db.MeliorSonusDatabase

actual fun createDatabaseFactoryTest(): SqlDriver{
    // Using ":memory:" tells the native C SQLite to run entirely in RAM
    return NativeSqliteDriver(MeliorSonusDatabase.Schema, "test.db")
}