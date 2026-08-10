package com.example.meliorsonus.util

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.pdf.PdfRenderer
import android.os.ParcelFileDescriptor
import android.content.Context
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import okio.FileSystem
import okio.Path
import okio.Path.Companion.toPath
import okio.buffer
import okio.openZip
import okio.use
import org.koin.core.context.GlobalContext

actual val systemFileSystem: FileSystem = FileSystem.SYSTEM

actual val tempDir: String by lazy {
    val context: Context = GlobalContext.get().get()
    context.cacheDir.absolutePath
}

actual val appFilesDir: String by lazy {
    val context: Context = GlobalContext.get().get()
    context.filesDir.absolutePath
}

actual val ioDispatcher: CoroutineDispatcher = Dispatchers.IO

actual fun unzipMusicXml(mxlPath: Path, xmlPath: Path) {
    val zipFileSystem = systemFileSystem.openZip(mxlPath)
    // Find first .xml file not in META-INF
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


