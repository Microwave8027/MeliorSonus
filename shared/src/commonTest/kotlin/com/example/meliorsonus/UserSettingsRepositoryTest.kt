package com.example.meliorsonus

import androidx.datastore.core.DataStoreFactory
import androidx.datastore.core.okio.OkioStorage
import okio.FileSystem
import okio.Path.Companion.toPath
import com.example.meliorsonus.datastore.UserSettingsSerializer
import com.example.meliorsonus.repository.UserSettingsRepositoryImpl
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull

class UserSettingsRepositoryTest {

    @Test
    fun testSaveAndRetrieveUserSettings() = runTest {
        val randomId = (1..100000).random()
        val testPath = FileSystem.SYSTEM.canonicalize(".".toPath()) / "build" / "tmp" / "test_user_settings_$randomId.pb"
        
        testPath.parent?.let { FileSystem.SYSTEM.createDirectories(it) }
        FileSystem.SYSTEM.delete(testPath, mustExist = false)

        val dataStore = DataStoreFactory.create(
            storage = OkioStorage(
                fileSystem = FileSystem.SYSTEM,
                serializer = UserSettingsSerializer,
                producePath = { testPath }
            )
        )

        val repository = UserSettingsRepositoryImpl(dataStore)

        val initialSettings = repository.userSettings.first()
        assertNotNull(initialSettings)
        assertEquals("", initialSettings.username)
        assertEquals("light", initialSettings.customization_theme)

        repository.updateUsername("TestUser")
        var updatedSettings = repository.userSettings.first()
        assertEquals("TestUser", updatedSettings.username)

        repository.updateTheme("dark")
        updatedSettings = repository.userSettings.first()
        assertEquals("dark", updatedSettings.customization_theme)
        assertEquals("TestUser", updatedSettings.username)

        FileSystem.SYSTEM.delete(testPath, mustExist = false)
    }
}
