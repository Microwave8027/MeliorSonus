package com.example.meliorsonus

import app.cash.sqldelight.driver.jdbc.sqlite.JdbcSqliteDriver
import com.example.meliorsonus.db.MeliorSonusDatabase
import com.example.meliorsonus.repository.SavedSheetRepositoryImpl
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class SavedSheetRepositoryTest {

    @Test
    fun testSavedSheetRepositoryOperations() = runTest {
        val driver = JdbcSqliteDriver(JdbcSqliteDriver.IN_MEMORY)
        MeliorSonusDatabase.Schema.create(driver)
        val database = MeliorSonusDatabase(driver)
        val repository = SavedSheetRepositoryImpl(database)

        // Ensure database is initially empty
        var savedSheets = repository.getAllSavedSheets()
        assertTrue(savedSheets.isEmpty())

        // Save a music sheet
        repository.saveSheet(
            mxl = "sheets/twinkle.mxl",
            title = "Twinkle Twinkle Little Star",
            artistName = "Traditional",
            composerName = "Mozart",
            publisher = "Public Domain",
            songLengthBars = 32L,
            genres = "Classical, Children",
            instruments = "Piano, Violin"
        )

        // Retrieve and verify
        savedSheets = repository.getAllSavedSheets()
        assertEquals(1, savedSheets.size)
        val sheet = savedSheets.first()
        assertEquals("sheets/twinkle.mxl", sheet.mxl)
        assertEquals("Twinkle Twinkle Little Star", sheet.title)
        assertEquals("Traditional", sheet.artist_name)
        assertEquals("Mozart", sheet.composer_name)
        assertEquals("Public Domain", sheet.publisher)
        assertEquals(32L, sheet.song_length_bars)
        assertEquals("Classical, Children", sheet.genres)
        assertEquals("Piano, Violin", sheet.instruments)
        assertTrue(sheet.saved_at > 0L)

        // Delete the saved sheet
        repository.deleteSheet("sheets/twinkle.mxl")

        // Retrieve and verify empty
        savedSheets = repository.getAllSavedSheets()
        assertTrue(savedSheets.isEmpty())
    }
}
