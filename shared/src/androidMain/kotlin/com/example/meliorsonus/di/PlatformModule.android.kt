package com.example.meliorsonus.di

import coil3.ImageLoader
import coil3.svg.SvgDecoder
import com.example.meliorsonus.datastore.DataStoreFactory
import com.example.meliorsonus.db.DatabaseDriverFactory
import com.example.meliorsonus.network.createHttpClient
import com.example.meliorsonus.infrastructure.VerovioBridge
import com.example.meliorsonus.util.SizeOfScreen
import com.example.meliorsonus.util.SizeOfScreenImpl
import org.koin.android.ext.koin.androidContext
import org.koin.core.module.Module
import org.koin.dsl.module

actual val platformModule: Module = module {
    single { createHttpClient }
    single { DataStoreFactory(get()).create() }
    single { DatabaseDriverFactory(get()).create() }
    single<SizeOfScreen> { SizeOfScreenImpl(get()) }
    single { VerovioBridge(androidContext()) }
    // Coil
    single<ImageLoader>{
        ImageLoader.Builder(get())
            .components {
                add(SvgDecoder.Factory())
            }
            .build()
    }
}
