package com.example.meliorsonus.di

import coil3.ImageLoader
import coil3.PlatformContext
import coil3.request.crossfade
import coil3.svg.SvgDecoder
import com.example.meliorsonus.datastore.DataStoreFactory
import com.example.meliorsonus.db.DatabaseDriverFactory
import com.example.meliorsonus.util.SizeOfScreen
import com.example.meliorsonus.util.SizeOfScreenImpl
import org.koin.core.module.Module
import org.koin.dsl.module

actual val platformModule: Module = module {
    single { DataStoreFactory().create() }
    // Provides SqlDriver; MeliorSonusDatabase is wired in databaseModule (nonWasmMain)
    single { DatabaseDriverFactory().create() }
    single<SizeOfScreen> { SizeOfScreenImpl() }
    single<PlatformContext> { PlatformContext.INSTANCE }
    // Coil
    single<ImageLoader> {
        ImageLoader.Builder(get())
            .components {
                add(SvgDecoder.Factory())
            }
            .crossfade(true)
            .build()
    }
}
