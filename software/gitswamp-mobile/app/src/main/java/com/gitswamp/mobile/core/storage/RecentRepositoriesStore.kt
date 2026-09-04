package com.gitswamp.mobile.core.storage

import android.content.Context
import androidx.core.content.edit
import com.gitswamp.mobile.core.model.RecentRepository
import org.json.JSONArray
import org.json.JSONObject

class RecentRepositoriesStore(context: Context) {
    private val preferences = context.getSharedPreferences("recent-repositories", Context.MODE_PRIVATE)

    fun read(): List<RecentRepository> {
        val raw = preferences.getString(KEY, null) ?: return emptyList()
        return runCatching {
            val array = JSONArray(raw)
            buildList {
                for (index in 0 until array.length()) {
                    val item = array.getJSONObject(index)
                    add(
                        RecentRepository(
                            name = item.getString("name"),
                            path = item.getString("path"),
                            branch = item.optString("branch", "HEAD"),
                            lastOpenedAt = item.optLong("lastOpenedAt", 0L),
                        ),
                    )
                }
            }
        }.getOrDefault(emptyList())
            .distinctBy { it.path }
            .sortedByDescending { it.lastOpenedAt }
            .take(MAX_RECENT)
    }

    fun touch(repository: RecentRepository) {
        val next = (listOf(repository) + read())
            .distinctBy { it.path }
            .sortedByDescending { it.lastOpenedAt }
            .take(MAX_RECENT)
        write(next)
    }

    fun remove(path: String) {
        write(read().filterNot { it.path == path })
    }

    fun clear() {
        preferences.edit { remove(KEY) }
    }

    private fun write(items: List<RecentRepository>) {
        val array = JSONArray()
        items.forEach { item ->
            array.put(
                JSONObject()
                    .put("name", item.name)
                    .put("path", item.path)
                    .put("branch", item.branch)
                    .put("lastOpenedAt", item.lastOpenedAt),
            )
        }
        preferences.edit { putString(KEY, array.toString()) }
    }

    private companion object {
        const val KEY = "repositories"
        const val MAX_RECENT = 24
    }
}
