package com.example.meliorsonus.di

import com.example.meliorsonus.db.MeliorSonusDatabase
import com.example.meliorsonus.repository.SavedSheetRepository
import com.example.meliorsonus.repository.SavedSheetRepositoryImpl
import org.koin.core.module.Module
import org.koin.dsl.module

val databaseModule: Module = module {
    single { MeliorSonusDatabase(get()) }
    single<SavedSheetRepository> { SavedSheetRepositoryImpl(get()) }
}
