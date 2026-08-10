package com.example.meliorsonus.di

import coil3.ImageLoader
import coil3.request.crossfade
import coil3.svg.SvgDecoder
import com.example.meliorsonus.domain.SaveAndDeletePDFUseCase
import com.example.meliorsonus.domain.SaveSheetUseCase
import com.example.meliorsonus.network.SheetSearchDataSource
import com.example.meliorsonus.repository.UserSettingsRepositoryImpl
import com.example.meliorsonus.repository.SheetSearchRepository
import com.example.meliorsonus.repository.SheetSearchRepositoryImpl
import com.example.meliorsonus.repository.UserSettingsRepository
import io.ktor.client.HttpClient
import org.koin.core.context.startKoin
import org.koin.dsl.KoinAppDeclaration
import org.koin.dsl.module

fun initKoin(appDeclaration: KoinAppDeclaration = {}) = startKoin {
    appDeclaration()
    modules(commonModule, platformModule, databaseModule)
}

val commonModule = module {
    // Data sources
    single { SheetSearchDataSource(get<HttpClient>()) }

    // Repositories (singletons per koin best practices)
    single<SheetSearchRepository> { SheetSearchRepositoryImpl(get()) }
    single<UserSettingsRepository> { UserSettingsRepositoryImpl(get()) }

    // Use cases
    factory { SaveSheetUseCase(get()) }
    factory { SaveAndDeletePDFUseCase(get()) }
}

