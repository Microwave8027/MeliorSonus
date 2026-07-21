package com.example.meliorsonus

import android.app.Application
import com.example.meliorsonus.di.initKoin
import org.koin.android.ext.koin.androidContext

class MeliorSonusApplication : Application() {
    override fun onCreate() {
        super.onCreate()
        initKoin {
            androidContext(this@MeliorSonusApplication)
        }
    }
}
