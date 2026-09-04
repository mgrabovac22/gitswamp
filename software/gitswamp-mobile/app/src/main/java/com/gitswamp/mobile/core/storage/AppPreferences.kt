package com.gitswamp.mobile.core.storage

import android.content.Context
import androidx.core.content.edit

class AppPreferences(context: Context) {
    private val preferences = context.getSharedPreferences("app-preferences", Context.MODE_PRIVATE)

    var useSystemTheme: Boolean
        get() = preferences.getBoolean(KEY_SYSTEM_THEME, false)
        set(value) = preferences.edit { putBoolean(KEY_SYSTEM_THEME, value) }

    var autoFetchOnOpen: Boolean
        get() = preferences.getBoolean(KEY_AUTO_FETCH, false)
        set(value) = preferences.edit { putBoolean(KEY_AUTO_FETCH, value) }

    var commitPageSize: Int
        get() = preferences.getInt(KEY_PAGE_SIZE, DEFAULT_PAGE_SIZE).coerceIn(100, 500)
        set(value) = preferences.edit { putInt(KEY_PAGE_SIZE, value.coerceIn(100, 500)) }

    private companion object {
        const val KEY_SYSTEM_THEME = "system-theme"
        const val KEY_AUTO_FETCH = "auto-fetch"
        const val KEY_PAGE_SIZE = "commit-page-size"
        const val DEFAULT_PAGE_SIZE = 200
    }
}
