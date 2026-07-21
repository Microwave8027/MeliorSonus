package com.example.meliorsonus.util

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import okio.FileSystem
import okio.Path
import okio.Path.Companion.toPath
import okio.buffer
import okio.openZip
import okio.use

actual val systemFileSystem: FileSystem = FileSystem.SYSTEM

actual val tempDir: String = System.getProperty("java.io.tmpdir") ?: "."

actual val appFilesDir: String = System.getProperty("user.home") ?: "."

actual val ioDispatcher: CoroutineDispatcher = Dispatchers.IO

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
