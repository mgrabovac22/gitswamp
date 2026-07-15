package com.gitswamp.mobile.di

import android.content.Context
import com.gitswamp.mobile.core.git.GitRepository
import com.gitswamp.mobile.core.git.JGitRepository
import com.gitswamp.mobile.core.storage.AppPreferences
import com.gitswamp.mobile.core.storage.RecentRepositoriesStore
import com.gitswamp.mobile.core.storage.RepositoryPathResolver
import com.gitswamp.mobile.core.storage.TokenVault

class AppContainer(context: Context) {
    private val applicationContext = context.applicationContext

    val gitRepository: GitRepository by lazy { JGitRepository() }
    val recentRepositories by lazy { RecentRepositoriesStore(applicationContext) }
    val tokenVault by lazy { TokenVault(applicationContext) }
    val pathResolver by lazy { RepositoryPathResolver(applicationContext) }
    val preferences by lazy { AppPreferences(applicationContext) }
}

