package com.example.meliorsonus.util

import kotlinx.coroutines.CoroutineDispatcher
import okio.FileSystem
import okio.Path

expect val systemFileSystem: FileSystem

expect val tempDir: String

expect val appFilesDir: String

expect val ioDispatcher: CoroutineDispatcher

expect fun unzipMusicXml(mxlPath: Path, xmlPath: Path)


