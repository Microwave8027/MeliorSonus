package com.example.meliorsonus.repository

import androidx.datastore.core.DataStore
import com.meliorsonus.app.UserSettings
import kotlinx.coroutines.flow.Flow

interface UserSettingsRepository {
    val userSettings: Flow<UserSettings>
    suspend fun updateUsername(username: String)
    suspend fun updateTheme(theme: String)
    suspend fun updateLanguage(lan: String)
}

class UserSettingsRepositoryImpl(
    private val dataStore: DataStore<UserSettings>
) : UserSettingsRepository {

    override val userSettings: Flow<UserSettings> = dataStore.data

    override suspend fun updateUsername(username: String) {
        dataStore.updateData { currentSettings ->
            currentSettings.copy(username = username,)
        }
    }

    override suspend fun updateTheme(theme: String) {
        dataStore.updateData { currentSettings ->
            currentSettings.copy(customization_theme = theme,)
        }
    }

    override suspend fun updateLanguage(lan: String) {
        dataStore.updateData { currentSettings ->
            currentSettings.copy(language = lan)
        }
    }
}
