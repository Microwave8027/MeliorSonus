package com.example.meliorsonus.datastore

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.core.DataStoreFactory
import androidx.datastore.core.okio.OkioStorage
import okio.FileSystem
import okio.Path.Companion.toPath
import com.meliorsonus.app.UserSettings

actual class DataStoreFactory(private val context: Context) {
    actual fun create(): DataStore<UserSettings> {
        return DataStoreFactory.create(
            storage = OkioStorage(
                fileSystem = FileSystem.SYSTEM,
                serializer = UserSettingsSerializer,
                producePath = {
                    val file = context.filesDir.resolve("user_settings.pb")
                    file.absolutePath.toPath()
                }
            )
        )
    }
}
