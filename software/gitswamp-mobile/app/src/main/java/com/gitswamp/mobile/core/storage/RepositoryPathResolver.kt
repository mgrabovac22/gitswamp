package com.gitswamp.mobile.core.storage

import android.content.Context
import android.net.Uri
import android.os.Environment
import android.provider.DocumentsContract
import androidx.documentfile.provider.DocumentFile
import com.gitswamp.mobile.core.model.CloneProgress
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.File
import java.io.FileOutputStream

class RepositoryPathResolver(private val context: Context) {
    val workspaceRoot: File by lazy {
        val external = context.getExternalFilesDir("repositories")
        (external ?: File(context.filesDir, "repositories")).apply { mkdirs() }
    }

    fun resolveTreeUri(uri: Uri): Result<File> = runCatching {
        if (uri.scheme == "file") {
            return@runCatching File(requireNotNull(uri.path)).canonicalFile
        }
        require(DocumentsContract.isTreeUri(uri)) { "Select a folder from local device storage." }
        require(uri.authority == EXTERNAL_STORAGE_AUTHORITY) {
            "This provider does not expose a local path. Clone the repository into GitSwamp instead."
        }
        val documentId = DocumentsContract.getTreeDocumentId(uri)
        val parts = documentId.split(':', limit = 2)
        val volume = parts.firstOrNull().orEmpty()
        val relative = parts.getOrElse(1) { "" }
        val root = if (volume.equals("primary", ignoreCase = true)) {
            Environment.getExternalStorageDirectory()
        } else {
            storageVolumeRoot(volume)
                ?: throw IllegalArgumentException("The selected storage volume is not mounted.")
        }
        File(root, relative).canonicalFile
    }

    suspend fun importTree(
        uri: Uri,
        onProgress: (CloneProgress) -> Unit,
    ): File = withContext(Dispatchers.IO) {
        val source = DocumentFile.fromTreeUri(context, uri)
            ?: throw IllegalArgumentException("The selected folder cannot be read.")
        require(source.isDirectory) { "Select a repository folder." }
        require(source.findFile(".git") != null) {
            "The selected provider did not expose a .git directory. Select the repository root from local storage."
        }

        val baseName = safeName(source.name ?: "imported-repository")
        val destination = uniqueDestination(baseName)
        var copiedFiles = 0
        onProgress(CloneProgress("Importing $baseName", 0, 0))
        try {
            destination.mkdirs()
            fun copyDirectory(directory: DocumentFile, target: File, depth: Int) {
                require(depth <= MAX_DEPTH) { "Repository folder nesting is too deep." }
                directory.listFiles().forEach { child ->
                    val childName = safeChildName(child.name ?: return@forEach)
                    val output = File(target, childName)
                    if (child.isDirectory) {
                        output.mkdirs()
                        copyDirectory(child, output, depth + 1)
                    } else if (child.isFile) {
                        val input = context.contentResolver.openInputStream(child.uri)
                            ?: throw IllegalStateException("Could not read ${child.name}.")
                        input.use { sourceStream ->
                            FileOutputStream(output).use { outputStream ->
                                sourceStream.copyTo(outputStream, DEFAULT_BUFFER_SIZE)
                            }
                        }
                        copiedFiles++
                        if (copiedFiles == 1 || copiedFiles % PROGRESS_BATCH == 0) {
                            onProgress(CloneProgress("Importing $baseName", copiedFiles, 0))
                        }
                    }
                }
            }
            copyDirectory(source, destination, 0)
            require(File(destination, ".git").exists()) { "The imported folder does not contain Git metadata." }
            onProgress(CloneProgress("Import complete", copiedFiles, copiedFiles))
            destination
        } catch (error: Throwable) {
            destination.deleteRecursively()
            throw error
        }
    }

    private fun storageVolumeRoot(volumeId: String): File? {
        val marker = "${File.separator}$volumeId${File.separator}"
        return context.externalCacheDirs
            .asSequence()
            .filterNotNull()
            .mapNotNull { cache ->
                val path = cache.absolutePath
                val index = path.indexOf(marker, ignoreCase = true)
                if (index < 0) null else File(path.substring(0, index + marker.length - 1))
            }
            .firstOrNull()
    }

    private fun uniqueDestination(baseName: String): File {
        val direct = File(workspaceRoot, baseName)
        if (!direct.exists()) return direct
        var suffix = 2
        while (suffix < 10_000) {
            val candidate = File(workspaceRoot, "$baseName-$suffix")
            if (!candidate.exists()) return candidate
            suffix++
        }
        throw IllegalStateException("Could not allocate a local repository folder.")
    }

    private fun safeName(name: String): String = name
        .replace(Regex("[^A-Za-z0-9._-]"), "-")
        .trim('-', '.')
        .ifBlank { "imported-repository" }
        .take(80)

    private fun safeChildName(name: String): String {
        require(name != "." && name != "..") { "Invalid repository entry." }
        return name.replace('/', '_').replace('\\', '_')
    }

    private companion object {
        const val EXTERNAL_STORAGE_AUTHORITY = "com.android.externalstorage.documents"
        const val MAX_DEPTH = 128
        const val PROGRESS_BATCH = 16
    }
}
