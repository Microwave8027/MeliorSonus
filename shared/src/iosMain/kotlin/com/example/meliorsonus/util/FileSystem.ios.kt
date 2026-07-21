package com.example.meliorsonus.util

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import okio.FileSystem
import okio.Path
import okio.Path.Companion.toPath
import okio.buffer
import okio.openZip
import okio.use
import platform.Foundation.NSTemporaryDirectory

actual val systemFileSystem: FileSystem = FileSystem.SYSTEM

actual val tempDir: String = NSTemporaryDirectory()

actual val appFilesDir: String = NSTemporaryDirectory()

actual val ioDispatcher: CoroutineDispatcher = Dispatchers.Default

actual fun unzipMusicXml(mxlPath: Path, xmlPath: Path) {
    val zipFileSystem = systemFileSystem.openZip(mxlPath)
    val musicXmlPath = zipFileSystem.list("/".toPath()).find { 
        it.name.endsWith(".xml") && !it.toString().contains("META-INF")
    } ?: zipFileSystem.list("/".toPath()).find { it.name.endsWith(".xml") }

    if (musicXmlPath != null) {
        zipFileSystem.source(musicXmlPath).buffer().use { source ->
            systemFileSystem.sink(xmlPath).buffer().use { sink ->
                sink.writeAll(source)
            }
        }
    }
    zipFileSystem.close()
}
