package com.example.meliorsonus


import app.cash.sqldelight.db.SqlDriver
import app.cash.sqldelight.driver.jdbc.sqlite.JdbcSqliteDriver
import com.example.meliorsonus.db.MeliorSonusDatabase

actual fun createDatabaseFactoryTest(): SqlDriver {
    val driver: SqlDriver = JdbcSqliteDriver(JdbcSqliteDriver.IN_MEMORY)
    MeliorSonusDatabase.Schema.create(driver)
    return driver
}