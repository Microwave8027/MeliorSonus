package com.example.meliorsonus.datastore

import androidx.datastore.core.DataStore
import androidx.datastore.core.DataStoreFactory
import androidx.datastore.core.okio.OkioStorage
import okio.FileSystem
import okio.Path.Companion.toPath
import com.meliorsonus.app.UserSettings

actual class DataStoreFactory {
    actual fun create(): DataStore<UserSettings> {
        return DataStoreFactory.create(
            storage = OkioStorage(
                fileSystem = FileSystem.SYSTEM,
                serializer = UserSettingsSerializer,
                producePath = {
                    val userHome = System.getProperty("user.home") ?: "."
                    "$userHome/.meliorsonus_user_settings.pb".toPath()
                }
            )
        )
    }
}
