package com.example.meliorsonus.util

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import okio.FileSystem
import okio.Path
import okio.Sink
import okio.Source
import okio.FileHandle
import okio.FileMetadata

private class DummyFileSystem : FileSystem() {
    override fun appendingSink(file: Path, mustExist: Boolean): Sink = throw UnsupportedOperationException()
    override fun atomicMove(source: Path, target: Path) = throw UnsupportedOperationException()
    override fun canonicalize(path: Path): Path = path
    override fun createDirectory(dir: Path, mustCreate: Boolean) = throw UnsupportedOperationException()
    override fun createSymlink(source: Path, target: Path) = throw UnsupportedOperationException()
    override fun delete(path: Path, mustExist: Boolean) = throw UnsupportedOperationException()
    override fun list(dir: Path): List<Path> = emptyList()
    override fun listOrNull(dir: Path): List<Path>? = null
    override fun metadataOrNull(path: Path): FileMetadata? = null
    override fun openReadOnly(file: Path): FileHandle = throw UnsupportedOperationException()
    override fun openReadWrite(file: Path, mustCreate: Boolean, mustExist: Boolean): FileHandle = throw UnsupportedOperationException()
    override fun sink(file: Path, mustCreate: Boolean): Sink = throw UnsupportedOperationException()
    override fun source(file: Path): Source = throw UnsupportedOperationException()
}

actual val systemFileSystem: FileSystem = DummyFileSystem()

actual val tempDir: String = "."

actual val appFilesDir: String = "."

actual val ioDispatcher: CoroutineDispatcher = Dispatchers.Default

actual fun unzipMusicXml(mxlPath: Path, xmlPath: Path) {
    // JS FileSystem is a dummy, unzipping is not supported in this mock
}
