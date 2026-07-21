package com.example.meliorsonus.datastore

import androidx.datastore.core.DataStore
import androidx.datastore.core.DataStoreFactory
import androidx.datastore.core.okio.OkioStorage
import okio.FileSystem
import okio.Path.Companion.toPath
import com.meliorsonus.app.UserSettings
import platform.Foundation.NSDocumentDirectory
import platform.Foundation.NSFileManager
import platform.Foundation.NSUserDomainMask
import platform.Foundation.NSURL

actual class DataStoreFactory {
    actual fun create(): DataStore<UserSettings> {
        return DataStoreFactory.create(
            storage = OkioStorage(
                fileSystem = FileSystem.SYSTEM,
                serializer = UserSettingsSerializer,
                producePath = {
                    val paths = NSFileManager.defaultManager.URLsForDirectory(NSDocumentDirectory, NSUserDomainMask)
                    val documentDirectory = paths.first() as NSURL
                    val path = documentDirectory.path + "/user_settings.pb"
                    path.toPath()
                }
            )
        )
    }
}
