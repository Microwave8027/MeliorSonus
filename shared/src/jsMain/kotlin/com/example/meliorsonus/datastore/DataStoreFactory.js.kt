package com.example.meliorsonus.datastore

import androidx.datastore.core.DataStore
import androidx.datastore.core.DataStoreFactory
import androidx.datastore.core.okio.OkioStorage
import okio.FileSystem
import okio.Path.Companion.toPath
import com.meliorsonus.app.UserSettings

private class DummyFileSystem : FileSystem() {
    override fun appendingSink(file: okio.Path, mustExist: Boolean): okio.Sink = throw UnsupportedOperationException()
    override fun atomicMove(source: okio.Path, target: okio.Path) = throw UnsupportedOperationException()
    override fun canonicalize(path: okio.Path): okio.Path = path
    override fun createDirectory(dir: okio.Path, mustCreate: Boolean) = throw UnsupportedOperationException()
    override fun createSymlink(source: okio.Path, target: okio.Path) = throw UnsupportedOperationException()
    override fun delete(path: okio.Path, mustExist: Boolean) = throw UnsupportedOperationException()
    override fun list(dir: okio.Path): List<okio.Path> = emptyList()
    override fun listOrNull(dir: okio.Path): List<okio.Path>? = null
    override fun metadataOrNull(path: okio.Path): okio.FileMetadata? = null
    override fun openReadOnly(file: okio.Path): okio.FileHandle = throw UnsupportedOperationException()
    override fun openReadWrite(file: okio.Path, mustCreate: Boolean, mustExist: Boolean): okio.FileHandle = throw UnsupportedOperationException()
    override fun sink(file: okio.Path, mustCreate: Boolean): okio.Sink = throw UnsupportedOperationException()
    override fun source(file: okio.Path): okio.Source = throw UnsupportedOperationException()
}

actual class DataStoreFactory {
    actual fun create(): DataStore<UserSettings> {
        return DataStoreFactory.create(
            storage = OkioStorage(
                fileSystem = DummyFileSystem(),
                serializer = UserSettingsSerializer,
                producePath = { "user_settings.pb".toPath() }
            )
        )
    }
}
