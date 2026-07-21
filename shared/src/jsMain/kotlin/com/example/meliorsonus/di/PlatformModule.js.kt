package com.example.meliorsonus.di

import com.example.meliorsonus.datastore.DataStoreFactory
import org.koin.core.module.Module
import org.koin.dsl.module

actual val platformModule: Module = module {
    single { DataStoreFactory().create() }
}
