package com.example.meliorsonus.util

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.cinterop.ExperimentalForeignApi
import kotlinx.cinterop.readBytes
import kotlinx.cinterop.useContents
import okio.FileSystem
import okio.Path
import okio.Path.Companion.toPath
import okio.buffer
import okio.openZip
import okio.use
import platform.CoreGraphics.CGBitmapContextCreate
import platform.CoreGraphics.CGBitmapContextCreateImage
import platform.CoreGraphics.CGContextDrawPDFPage
import platform.CoreGraphics.CGContextFillRect
import platform.CoreGraphics.CGContextScaleCTM
import platform.CoreGraphics.CGContextSetRGBFillColor
import platform.CoreGraphics.CGContextTranslateCTM
import platform.CoreGraphics.CGColorSpaceCreateDeviceRGB
import platform.CoreGraphics.CGDataProviderCreateWithFilename
import platform.CoreGraphics.CGImageAlphaInfo
import platform.CoreGraphics.CGPDFBox
import platform.CoreGraphics.CGPDFDocumentCreateWithProvider
import platform.CoreGraphics.CGPDFDocumentGetNumberOfPages
import platform.CoreGraphics.CGPDFDocumentGetPage
import platform.CoreGraphics.CGPDFPageGetBoxRect
import platform.CoreGraphics.kCGPDFMediaBox
import platform.Foundation.NSTemporaryDirectory
import platform.UIKit.UIImage
import platform.UIKit.UIImagePNGRepresentation

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


