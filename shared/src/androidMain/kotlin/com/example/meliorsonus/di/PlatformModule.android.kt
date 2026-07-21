package com.example.meliorsonus.di

import com.example.meliorsonus.datastore.DataStoreFactory
import com.example.meliorsonus.db.DatabaseDriverFactory
import com.example.meliorsonus.network.createHttpClient
import org.koin.core.module.Module
import org.koin.dsl.module

actual val platformModule: Module = module {
    single { createHttpClient }
    single { DataStoreFactory(get()).create() }
    single { DatabaseDriverFactory(get()).create() }
}
