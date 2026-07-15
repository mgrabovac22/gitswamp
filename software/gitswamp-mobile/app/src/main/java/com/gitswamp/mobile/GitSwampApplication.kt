package com.gitswamp.mobile

import android.app.Application
import com.gitswamp.mobile.di.AppContainer

class GitSwampApplication : Application() {
    val container: AppContainer by lazy { AppContainer(this) }
}

