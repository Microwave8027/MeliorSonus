package com.example.meliorsonus.di

import coil3.ImageLoader
import coil3.request.crossfade
import coil3.svg.SvgDecoder
import com.example.meliorsonus.domain.GetProportionalHeightUseCase
import com.example.meliorsonus.domain.GetSVGUseCase
import com.example.meliorsonus.domain.SaveSheetUseCase
import com.example.meliorsonus.infrastructure.VerovioBridge
import com.example.meliorsonus.network.SheetSearchDataSource
import com.example.meliorsonus.repository.UserSettingsRepositoryImpl
import com.example.meliorsonus.repository.SheetSearchRepository
import com.example.meliorsonus.repository.SheetSearchRepositoryImpl
import com.example.meliorsonus.repository.UserSettingsRepository
import com.example.meliorsonus.repository.ZoomSettingsRepository
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
    single { ZoomSettingsRepository(get()) }

    // Use cases
    factory { SaveSheetUseCase(get(), get(), get()) }
    factory { GetSVGUseCase(get()) }
    factory { GetProportionalHeightUseCase(get()) }
}

