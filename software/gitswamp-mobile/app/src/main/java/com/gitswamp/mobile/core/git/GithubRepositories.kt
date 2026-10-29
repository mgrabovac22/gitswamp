package com.gitswamp.mobile.core.git

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONArray
import java.net.HttpURLConnection
import java.net.URI
import java.net.URL

data class GithubRepository(val id: Long, val fullName: String, val cloneUrl: String, val private: Boolean) {
    val name: String get() = fullName.substringAfterLast('/')

    companion object {
        fun create(id: Long, fullName: String, cloneUrl: String, private: Boolean): GithubRepository? {
            val uri = runCatching { URI(cloneUrl) }.getOrNull() ?: return null
            if (id <= 0 || !fullName.matches(Regex("[^/]+/[^/]+")) || uri.scheme != "https" ||
                !uri.host.equals("github.com", true) || uri.userInfo != null || uri.port != -1 ||
                uri.query != null || uri.fragment != null || uri.path != "/$fullName.git") return null
            return GithubRepository(id, fullName, cloneUrl, private)
        }
    }
}

data class GithubRepositoryPage(val repositories: List<GithubRepository>, val hasMore: Boolean)

object GithubRepositories {
    suspend fun fetch(token: String, page: Int): GithubRepositoryPage = withContext(Dispatchers.IO) {
        require(token.isNotBlank() && page > 0)
        val connection = URL("https://api.github.com/user/repos?per_page=50&page=$page&sort=updated&affiliation=owner,collaborator,organization_member")
            .openConnection() as HttpURLConnection
        try {
            connection.instanceFollowRedirects = false
            connection.connectTimeout = 15000
            connection.readTimeout = 20000
            connection.setRequestProperty("Authorization", "Bearer $token")
            connection.setRequestProperty("Accept", "application/vnd.github+json")
            connection.setRequestProperty("User-Agent", "GitSwamp-Mobile")
            val status = connection.responseCode
            check(status == 200) {
                if (status == 401 || status == 403) "GitHub denied repository access. Check account permissions or reconnect."
                else "GitHub repository list is unavailable (HTTP $status)."
            }
            val array = connection.inputStream.bufferedReader().use { JSONArray(it.readText()) }
            val repos = (0 until array.length()).mapNotNull { index ->
                val item = array.getJSONObject(index)
                GithubRepository.create(item.getLong("id"), item.getString("full_name"), item.getString("clone_url"), item.optBoolean("private"))
            }
            GithubRepositoryPage(repos, connection.getHeaderField("Link").orEmpty().contains("rel=\"next\""))
        } finally { connection.disconnect() }
    }
}
