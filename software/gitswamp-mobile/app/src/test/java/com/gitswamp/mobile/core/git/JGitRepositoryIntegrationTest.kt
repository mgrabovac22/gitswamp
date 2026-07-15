package com.gitswamp.mobile.core.git

import com.gitswamp.mobile.core.model.CloneRequest
import com.gitswamp.mobile.core.model.RemoteAction
import kotlinx.coroutines.runBlocking
import org.eclipse.jgit.storage.file.FileRepositoryBuilder
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import java.io.File

class JGitRepositoryIntegrationTest {
    @get:Rule
    val temporaryFolder = TemporaryFolder()

    private val git = JGitRepository()

    @Test
    fun `discard restores index content and preserves staged changes`() = runBlocking {
        val repository = createRepository("discard-test")
        val file = File(repository, "notes.txt")
        file.writeText("base\n")
        git.stage(repository, listOf("notes.txt"))
        git.commit(repository, "Initial commit")

        file.writeText("staged version\n")
        git.stage(repository, listOf("notes.txt"))
        file.writeText("unstaged version\n")

        val before = git.loadStatus(repository)
        assertTrue(before.staged.any { it.path == "notes.txt" })
        assertTrue(before.unstaged.any { it.path == "notes.txt" })

        git.discardUnstaged(repository, listOf("notes.txt"))

        val after = git.loadStatus(repository)
        assertEquals("staged version\n", file.readText().replace("\r\n", "\n"))
        assertTrue(after.staged.any { it.path == "notes.txt" })
        assertFalse(after.unstaged.any { it.path == "notes.txt" })
    }

    @Test
    fun `unstaging a new file keeps it in the working tree`() = runBlocking {
        val repository = createRepository("unstage-test")
        val file = File(repository, "draft.txt")
        file.writeText("draft\n")

        git.stage(repository, listOf("draft.txt"))
        git.unstage(repository, listOf("draft.txt"))

        val status = git.loadStatus(repository)
        assertTrue(file.exists())
        assertTrue(status.staged.none { it.path == "draft.txt" })
        assertTrue(status.unstaged.any { it.path == "draft.txt" })
    }

    @Test
    fun `local clone loads graph and initial file diff`() = runBlocking {
        val source = createRepository("source")
        File(source, "README.md").writeText("# Mobile\n")
        git.stage(source, listOf("README.md"))
        val sha = git.commit(source, "Add README")

        val cloneRoot = temporaryFolder.newFolder("clones")
        val clone = git.clone(
            CloneRequest(source.toURI().toString(), "cloned", "", "", false),
            cloneRoot,
        ) { }

        val snapshot = git.loadSnapshot(clone, 20)
        val diff = git.loadCommitDiff(clone, sha, "README.md")
        assertEquals("Add README", snapshot.commits.first().message)
        assertTrue(diff.lines.any { it.text == "# Mobile" })
    }

    @Test
    fun `file preview rejects paths outside repository`() = runBlocking {
        val repository = createRepository("path-test")
        val result = runCatching { git.readWorkingFile(repository, "../outside.txt") }
        assertTrue(result.isFailure)
    }

    @Test
    fun `clone rejects unencrypted http remote before network access`() = runBlocking {
        val cloneRoot = temporaryFolder.newFolder("insecure-clones")
        val result = runCatching {
            git.clone(
                CloneRequest("http://example.invalid/repository.git", "unsafe", "", "", false),
                cloneRoot,
            ) { }
        }

        assertTrue(result.isFailure)
        assertTrue(result.exceptionOrNull()?.message.orEmpty().contains("HTTPS remotes"))
        assertFalse(File(cloneRoot, "unsafe").exists())
    }

    @Test
    fun `remote action rejects disabled tls verification before network access`() = runBlocking {
        val repository = createRepository("tls-test")
        FileRepositoryBuilder().setGitDir(File(repository, ".git")).setWorkTree(repository).build().use { handle ->
            handle.config.setString("remote", "origin", "url", "https://example.invalid/repository.git")
            handle.config.setBoolean("http", null, "sslVerify", false)
            handle.config.save()
        }

        val result = runCatching { git.runRemoteAction(repository, RemoteAction.Fetch, "", "") }
        assertTrue(result.isFailure)
        assertTrue(result.exceptionOrNull()?.message.orEmpty().contains("certificate verification"))
    }

    @Test
    fun `local push updates remote and fetch prunes deleted remote branch`() = runBlocking {
        val source = createRepository("remote-source")
        File(source, "README.md").writeText("initial\n")
        git.stage(source, listOf("README.md"))
        git.commit(source, "Initial remote commit")

        val bareRemote = File(temporaryFolder.root, "mobile-remote.git")
        org.eclipse.jgit.api.Git.cloneRepository()
            .setURI(source.toURI().toString())
            .setDirectory(bareRemote)
            .setBare(true)
            .call()
            .use { }

        val cloneRoot = temporaryFolder.newFolder("mobile-clones")
        val mobile = git.clone(
            CloneRequest(bareRemote.toURI().toString(), "mobile", "", "", false),
            cloneRoot,
        ) { }
        File(mobile, "README.md").appendText("from mobile\n")
        git.stage(mobile, listOf("README.md"))
        val pushedSha = git.commit(mobile, "Update from mobile")
        git.runRemoteAction(mobile, RemoteAction.Push, "", "")

        FileRepositoryBuilder().setGitDir(bareRemote).setBare().build().use { remote ->
            val branch = git.loadSnapshot(mobile, 20).currentBranch
            assertEquals(pushedSha, remote.resolve("refs/heads/$branch").name())
            remote.updateRef("refs/heads/obsolete").apply {
                setNewObjectId(remote.resolve("refs/heads/$branch"))
                update()
            }
        }

        git.runRemoteAction(mobile, RemoteAction.Fetch, "", "")
        assertTrue(git.loadBranches(mobile).any { it.name == "origin/obsolete" })

        FileRepositoryBuilder().setGitDir(bareRemote).setBare().build().use { remote ->
            remote.updateRef("refs/heads/obsolete").apply {
                setForceUpdate(true)
                delete()
            }
            assertEquals(null, remote.exactRef("refs/heads/obsolete"))
        }
        git.runRemoteAction(mobile, RemoteAction.Fetch, "", "")
        assertTrue(git.loadBranches(mobile).none { it.name == "origin/obsolete" })
    }

    private suspend fun createRepository(name: String): File {
        val root = temporaryFolder.newFolder("root-$name")
        val repository = git.init(root, name)
        FileRepositoryBuilder().setGitDir(File(repository, ".git")).setWorkTree(repository).build().use { handle ->
            handle.config.setString("user", null, "name", "GitSwamp Test")
            handle.config.setString("user", null, "email", "test@gitswamp.local")
            handle.config.save()
        }
        return repository
    }
}
