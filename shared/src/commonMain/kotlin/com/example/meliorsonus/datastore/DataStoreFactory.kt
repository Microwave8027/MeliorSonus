package com.example.meliorsonus.datastore

import androidx.datastore.core.DataStore
import com.meliorsonus.app.UserSettings

expect class DataStoreFactory {
    fun create(): DataStore<UserSettings>
}
