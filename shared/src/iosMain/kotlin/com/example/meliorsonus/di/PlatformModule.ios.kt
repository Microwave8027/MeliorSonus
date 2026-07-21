package com.example.meliorsonus.di

import com.example.meliorsonus.datastore.DataStoreFactory
import com.example.meliorsonus.db.DatabaseDriverFactory
import org.koin.core.module.Module
import org.koin.dsl.module

actual val platformModule: Module = module {
    single { DataStoreFactory().create() }
    // Provides SqlDriver; MeliorSonusDatabase is wired in databaseModule (nonWasmMain)
    single { DatabaseDriverFactory().create() }
}
