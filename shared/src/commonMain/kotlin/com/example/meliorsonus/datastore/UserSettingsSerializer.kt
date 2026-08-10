package com.example.meliorsonus.datastore

import androidx.datastore.core.okio.OkioSerializer
import okio.BufferedSink
import okio.BufferedSource
import com.meliorsonus.app.UserSettings

object UserSettingsSerializer : OkioSerializer<UserSettings> {
    override val defaultValue: UserSettings = UserSettings(
        username = "",
        customization_theme = "light",
        language = "english"
    )

    override suspend fun readFrom(source: BufferedSource): UserSettings {
        return try {
            UserSettings.ADAPTER.decode(source)
        } catch (e: Exception) {
            defaultValue
        }
    }

    override suspend fun writeTo(t: UserSettings, sink: BufferedSink) {
        UserSettings.ADAPTER.encode(sink, t)
    }
}
