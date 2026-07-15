package com.gitswamp.mobile.core.git

import com.gitswamp.mobile.core.model.BranchInfo
import com.gitswamp.mobile.core.model.AuthorIdentity
import com.gitswamp.mobile.core.model.ChangeKind
import com.gitswamp.mobile.core.model.ChangedFile
import com.gitswamp.mobile.core.model.CloneProgress
import com.gitswamp.mobile.core.model.CloneRequest
import com.gitswamp.mobile.core.model.CommitDetails
import com.gitswamp.mobile.core.model.CommitFile
import com.gitswamp.mobile.core.model.CommitInfo
import com.gitswamp.mobile.core.model.DiffLine
import com.gitswamp.mobile.core.model.DiffLineKind
import com.gitswamp.mobile.core.model.FileDiff
import com.gitswamp.mobile.core.model.RemoteAction
import com.gitswamp.mobile.core.model.RepositoryEntry
import com.gitswamp.mobile.core.model.RepositorySnapshot
import com.gitswamp.mobile.core.model.RepositoryStatus
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.eclipse.jgit.api.CreateBranchCommand
import org.eclipse.jgit.api.Git
import org.eclipse.jgit.api.ListBranchCommand
import org.eclipse.jgit.api.ResetCommand
import org.eclipse.jgit.dircache.DirCacheIterator
import org.eclipse.jgit.diff.DiffEntry
import org.eclipse.jgit.diff.DiffFormatter
import org.eclipse.jgit.diff.RawText
import org.eclipse.jgit.lib.BranchTrackingStatus
import org.eclipse.jgit.lib.Constants
import org.eclipse.jgit.lib.ObjectId
import org.eclipse.jgit.lib.ProgressMonitor
import org.eclipse.jgit.lib.Ref
import org.eclipse.jgit.lib.Repository
import org.eclipse.jgit.revwalk.RevCommit
import org.eclipse.jgit.revwalk.RevSort
import org.eclipse.jgit.revwalk.RevWalk
import org.eclipse.jgit.storage.file.FileRepositoryBuilder
import org.eclipse.jgit.transport.CredentialsProvider
import org.eclipse.jgit.transport.RemoteRefUpdate
import org.eclipse.jgit.transport.TagOpt
import org.eclipse.jgit.transport.UsernamePasswordCredentialsProvider
import org.eclipse.jgit.treewalk.AbstractTreeIterator
import org.eclipse.jgit.treewalk.CanonicalTreeParser
import org.eclipse.jgit.treewalk.EmptyTreeIterator
import org.eclipse.jgit.treewalk.FileTreeIterator
import org.eclipse.jgit.treewalk.filter.PathFilter
import org.eclipse.jgit.util.io.DisabledOutputStream
import java.io.ByteArrayOutputStream
import java.io.File
import java.io.InputStream
import java.net.URI
import java.nio.charset.CodingErrorAction
import java.time.Instant
import kotlin.math.min

class JGitRepository : GitRepository {
    override suspend fun clone(
        request: CloneRequest,
        destinationRoot: File,
        onProgress: (CloneProgress) -> Unit,
    ): File = io {
        require(request.url.isNotBlank()) { "Repository URL is required." }
        requireSupportedRemote(request.url)
        destinationRoot.mkdirs()
        val safeName = safeRepositoryName(request.destinationName.ifBlank { nameFromUrl(request.url) })
        val destination = File(destinationRoot, safeName)
        require(!destination.exists()) { "A repository named '$safeName' already exists on this device." }

        try {
            val command = Git.cloneRepository()
                .setURI(request.url.trim())
                .setDirectory(destination)
                .setCloneAllBranches(true)
                .setProgressMonitor(CallbackProgressMonitor(onProgress))
            credentials(request.username, request.token)?.let(command::setCredentialsProvider)
            command.call().use { }
            destination
        } catch (error: Throwable) {
            destination.deleteRecursively()
            throw GitOperationException(cleanMessage(error, request.token), error)
        }
    }

    override suspend fun init(destinationRoot: File, name: String): File = io {
        destinationRoot.mkdirs()
        val safeName = safeRepositoryName(name)
        val destination = File(destinationRoot, safeName)
        require(!destination.exists()) { "A repository named '$safeName' already exists on this device." }
        Git.init().setDirectory(destination).call().use { }
        destination
    }

    override suspend fun validate(path: File): Result<Unit> = io {
        runCatching {
            openRepository(path).use { repository ->
                require(!repository.isBare) { "Bare repositories are not supported in the mobile workspace." }
            }
        }
    }

    override suspend fun loadSnapshot(path: File, commitLimit: Int): RepositorySnapshot = io {
        openRepository(path).use { repository ->
            val (commits, hasMore) = commits(repository, commitLimit)
            RepositorySnapshot(
                name = path.name,
                path = path.absolutePath,
                currentBranch = currentBranch(repository),
                remoteUrl = repository.config.getString("remote", "origin", "url"),
                identity = identity(repository),
                commits = commits,
                branches = branches(repository),
                status = status(repository),
                hasMoreCommits = hasMore,
            )
        }
    }

    override suspend fun loadCommits(path: File, limit: Int): Pair<List<CommitInfo>, Boolean> = io {
        openRepository(path).use { commits(it, limit) }
    }

    override suspend fun loadStatus(path: File): RepositoryStatus = io {
        openRepository(path).use(::status)
    }

    override suspend fun loadBranches(path: File): List<BranchInfo> = io {
        openRepository(path).use(::branches)
    }

    override suspend fun loadCommitDetails(path: File, sha: String): CommitDetails = io {
        openRepository(path).use { repository ->
            RevWalk(repository).use { walk ->
                val commit = walk.parseCommit(requireObjectId(repository, sha))
                val refsByCommit = refsByCommit(repository)
                val files = scanCommitDiff(repository, walk, commit).map { entry ->
                    val edits = DiffFormatter(DisabledOutputStream.INSTANCE).use { formatter ->
                        formatter.setRepository(repository)
                        formatter.toFileHeader(entry).toEditList()
                    }
                    CommitFile(
                        path = displayPath(entry),
                        oldPath = entry.oldPath.takeUnless { it == DiffEntry.DEV_NULL || it == entry.newPath },
                        kind = entry.changeType.toKind(),
                        additions = edits.sumOf { it.endB - it.beginB },
                        deletions = edits.sumOf { it.endA - it.beginA },
                    )
                }
                CommitDetails(commit.toModel(refsByCommit[commit.id.name()].orEmpty()), files)
            }
        }
    }

    override suspend fun loadCommitDiff(path: File, sha: String, filePath: String): FileDiff = io {
        openRepository(path).use { repository ->
            RevWalk(repository).use { walk ->
                val commit = walk.parseCommit(requireObjectId(repository, sha))
                val entries = scanCommitDiff(repository, walk, commit, filePath)
                val entry = entries.firstOrNull { displayPath(it) == filePath || it.oldPath == filePath }
                    ?: throw GitOperationException("No changes for '$filePath' in this commit.")
                formatDiff(repository, entry)
            }
        }
    }

    override suspend fun loadWorkingDiff(path: File, filePath: String, staged: Boolean): FileDiff = io {
        openRepository(path).use { repository ->
            val oldTree: AbstractTreeIterator
            val newTree: AbstractTreeIterator
            if (staged) {
                oldTree = headTreeParser(repository)
                newTree = DirCacheIterator(repository.readDirCache())
            } else {
                oldTree = DirCacheIterator(repository.readDirCache())
                newTree = FileTreeIterator(repository)
            }

            val scannedDiff = DiffFormatter(DisabledOutputStream.INSTANCE).use { formatter ->
                formatter.setRepository(repository)
                formatter.setDetectRenames(true)
                formatter.pathFilter = PathFilter.create(filePath)
                val entry = formatter.scan(oldTree, newTree)
                    .firstOrNull { displayPath(it) == filePath || it.oldPath == filePath }
                entry?.let { formatDiff(repository, it) }
            }
            if (scannedDiff != null) return@io scannedDiff

            val current = File(repository.workTree, filePath)
            if (!staged && current.isFile) {
                val bytes = current.inputStream().use { it.readUpTo(MAX_DIFF_BYTES + 1) }
                val truncated = bytes.size > MAX_DIFF_BYTES
                val visible = bytes.copyOf(min(bytes.size, MAX_DIFF_BYTES))
                if (RawText.isBinary(visible)) {
                    FileDiff(filePath, null, emptyList(), truncated, binary = true)
                } else {
                    val text = decodeText(visible)
                    FileDiff(
                        path = filePath,
                        oldPath = null,
                        lines = text.lineSequence().take(MAX_DIFF_LINES).mapIndexed { index, line ->
                            DiffLine(DiffLineKind.Added, line, null, index + 1)
                        }.toList(),
                        truncated = truncated || text.lineSequence().count() > MAX_DIFF_LINES,
                        binary = false,
                    )
                }
            } else {
                throw GitOperationException("No ${if (staged) "staged" else "unstaged"} diff for '$filePath'.")
            }
        }
    }

    override suspend fun listTree(path: File, relativePath: String): List<RepositoryEntry> = io {
        val root = path.canonicalFile
        val directory = resolveInside(root, relativePath)
        require(directory.isDirectory) { "Folder does not exist." }
        directory.listFiles().orEmpty()
            .asSequence()
            .filterNot { it.name == ".git" }
            .sortedWith(compareByDescending<File> { it.isDirectory }.thenBy { it.name.lowercase() })
            .map {
                RepositoryEntry(
                    name = it.name,
                    relativePath = it.relativeTo(root).invariantSeparatorsPath,
                    isDirectory = it.isDirectory,
                    sizeBytes = if (it.isFile) it.length() else 0L,
                )
            }
            .toList()
    }

    override suspend fun readWorkingFile(path: File, relativePath: String): String = io {
        val file = resolveInside(path.canonicalFile, relativePath)
        require(file.isFile) { "File does not exist." }
        val bytes = file.inputStream().use { it.readUpTo(MAX_FILE_BYTES + 1) }
        require(bytes.size <= MAX_FILE_BYTES) { "File is larger than the mobile preview limit." }
        require(!RawText.isBinary(bytes)) { "Binary files cannot be previewed as text." }
        decodeText(bytes)
    }

    override suspend fun stage(path: File, filePaths: List<String>) = io {
        require(filePaths.isNotEmpty()) { "Select at least one file." }
        openRepository(path).use { repository ->
            Git(repository).use { git ->
                filePaths.distinct().forEach { filePath ->
                    git.add().addFilepattern(filePath).call()
                    if (!File(repository.workTree, filePath).exists()) {
                        git.add().setUpdate(true).addFilepattern(filePath).call()
                    }
                }
            }
        }
    }

    override suspend fun unstage(path: File, filePaths: List<String>): Unit = io {
        require(filePaths.isNotEmpty()) { "Select at least one file." }
        openRepository(path).use { repository ->
            Git(repository).use { git ->
                if (repository.resolve(Constants.HEAD) == null) {
                    filePaths.distinct().forEach { git.rm().setCached(true).addFilepattern(it).call() }
                } else {
                    val reset = git.reset().setMode(ResetCommand.ResetType.MIXED)
                    filePaths.distinct().forEach(reset::addPath)
                    reset.call()
                }
            }
        }
        Unit
    }

    override suspend fun discardUnstaged(path: File, filePaths: List<String>) = io {
        require(filePaths.isNotEmpty()) { "Select at least one file." }
        openRepository(path).use { repository ->
            val currentStatus = status(repository)
            val unstagedByPath = currentStatus.unstaged.associateBy { it.path }
            val conflicts = currentStatus.conflicts.mapTo(hashSetOf()) { it.path }
            val selected = filePaths.distinct()
            require(selected.none(conflicts::contains)) { "Resolve conflicts before discarding changes." }

            Git(repository).use { git ->
                selected.forEach { filePath ->
                    val change = unstagedByPath[filePath] ?: return@forEach
                    if (change.kind == ChangeKind.Untracked) {
                        deleteInside(repository.workTree.canonicalFile, filePath)
                    } else {
                        git.checkout().addPath(filePath).call()
                    }
                }
            }
        }
    }

    override suspend fun commit(path: File, message: String): String = io {
        require(message.trim().isNotEmpty()) { "Commit message is required." }
        openRepository(path).use { repository ->
            Git(repository).use { git ->
                git.commit().setMessage(message.trim()).call().id.name()
            }
        }
    }

    override suspend fun saveIdentity(path: File, identity: AuthorIdentity): Unit = io {
        require(identity.name.trim().isNotEmpty()) { "Author name is required." }
        require(identity.email.trim().contains('@')) { "Enter a valid author email." }
        openRepository(path).use { repository ->
            repository.config.setString("user", null, "name", identity.name.trim())
            repository.config.setString("user", null, "email", identity.email.trim())
            repository.config.save()
        }
    }

    override suspend fun checkoutBranch(path: File, branchName: String): Unit = io {
        val requested = branchName.trim().removePrefix("refs/heads/")
        require(requested.isNotEmpty()) { "Branch name is required." }
        openRepository(path).use { repository ->
            Git(repository).use { git ->
                val localRef = repository.findRef("refs/heads/$requested")
                if (localRef != null) {
                    git.checkout().setName(requested).call()
                } else {
                    val remoteName = requested.removePrefix("refs/remotes/")
                    val remoteRef = repository.findRef("refs/remotes/$remoteName")
                        ?: repository.findRef(remoteName)
                        ?: throw GitOperationException("Branch '$branchName' was not found.")
                    val localName = remoteName.substringAfter('/')
                    git.checkout()
                        .setCreateBranch(true)
                        .setName(localName)
                        .setStartPoint(remoteRef.name)
                        .setUpstreamMode(CreateBranchCommand.SetupUpstreamMode.TRACK)
                        .call()
                }
            }
        }
        Unit
    }

    override suspend fun createBranch(path: File, branchName: String): Unit = io {
        val name = branchName.trim()
        require(name.isNotEmpty()) { "Branch name is required." }
        openRepository(path).use { repository ->
            Git(repository).use { git ->
                git.checkout().setCreateBranch(true).setName(name).call()
            }
        }
        Unit
    }

    override suspend fun runRemoteAction(
        path: File,
        action: RemoteAction,
        username: String,
        token: String,
    ): String = io {
        openRepository(path).use { repository ->
            requireSupportedRemote(repository.config.getString("remote", "origin", "url").orEmpty())
            requireTlsVerification(repository)
            Git(repository).use { git ->
                val provider = credentials(username, token)
                when (action) {
                    RemoteAction.Fetch -> {
                        val command = git.fetch().setRemote("origin").setRemoveDeletedRefs(true).setTagOpt(TagOpt.FETCH_TAGS)
                        provider?.let(command::setCredentialsProvider)
                        val result = command.call()
                        result.messages.ifBlank { "Fetch completed." }
                    }

                    RemoteAction.Pull -> {
                        val command = git.pull().setRemote("origin")
                        provider?.let(command::setCredentialsProvider)
                        val result = command.call()
                        if (result.isSuccessful) "Pull completed." else "Pull finished with conflicts or an incomplete merge."
                    }

                    RemoteAction.Push -> {
                        val command = git.push().setRemote("origin")
                        provider?.let(command::setCredentialsProvider)
                        val updates = command.call().flatMap { it.remoteUpdates }
                        val failed = updates.filter {
                            it.status != RemoteRefUpdate.Status.OK && it.status != RemoteRefUpdate.Status.UP_TO_DATE
                        }
                        if (failed.isNotEmpty()) {
                            throw GitOperationException(failed.joinToString { "${it.remoteName}: ${it.status}" })
                        }
                        "Push completed."
                    }
                }
            }
        }
    }

    private fun openRepository(path: File): Repository {
        require(path.exists()) { "Repository folder does not exist." }
        require(File(path, Constants.DOT_GIT).exists()) { "The selected folder is not a Git repository root." }
        return FileRepositoryBuilder()
            .setWorkTree(path)
            .readEnvironment()
            .findGitDir(path)
            .build()
            .also { require(it.objectDatabase.exists()) { "The selected folder is not a Git repository." } }
    }

    private fun requireSupportedRemote(remoteUrl: String) {
        val scheme = runCatching { URI(remoteUrl.trim()).scheme?.lowercase() }.getOrNull()
        require(scheme == "https" || scheme == "file") {
            "GitSwamp Mobile supports HTTPS remotes. SSH and unencrypted HTTP remotes are not enabled."
        }
    }

    private fun requireTlsVerification(repository: Repository) {
        val remoteUrl = repository.config.getString("remote", "origin", "url").orEmpty()
        if (!remoteUrl.startsWith("https://", ignoreCase = true)) return

        val scopes = listOf<String?>(null) + repository.config.getSubsections("http")
        val verificationDisabled = scopes.any { subsection ->
            repository.config.getString("http", subsection, "sslVerify") != null &&
                !repository.config.getBoolean("http", subsection, "sslVerify", true)
        }
        require(!verificationDisabled) {
            "TLS certificate verification is disabled in this repository. Enable http.sslVerify before using remote actions."
        }
    }

    private fun commits(repository: Repository, requestedLimit: Int): Pair<List<CommitInfo>, Boolean> {
        val limit = requestedLimit.coerceIn(1, MAX_COMMITS)
        val refsByCommit = refsByCommit(repository)
        val starts = repository.refDatabase
            .getRefsByPrefix(Constants.R_HEADS)
            .plus(repository.refDatabase.getRefsByPrefix(Constants.R_REMOTES))
            .mapNotNull(Ref::getObjectId)
            .distinctBy(ObjectId::name)
        if (starts.isEmpty()) return emptyList<CommitInfo>() to false

        RevWalk(repository).use { walk ->
            walk.sort(RevSort.TOPO)
            walk.sort(RevSort.COMMIT_TIME_DESC, true)
            starts.forEach { objectId -> runCatching { walk.markStart(walk.parseCommit(objectId)) } }
            val values = ArrayList<CommitInfo>(limit + 1)
            val iterator = walk.iterator()
            while (iterator.hasNext() && values.size <= limit) {
                val commit = iterator.next()
                values += commit.toModel(refsByCommit[commit.id.name()].orEmpty())
            }
            val hasMore = values.size > limit
            if (hasMore) values.removeAt(values.lastIndex)
            return values to hasMore
        }
    }

    private fun status(repository: Repository): RepositoryStatus {
        val result = Git.wrap(repository).status().call()
        val conflicts = result.conflicting
            .map { ChangedFile(it, ChangeKind.Conflicted, staged = false) }
            .sortedBy { it.path.lowercase() }
        val conflictPaths = result.conflicting.toHashSet()
        val staged = buildList {
            result.added.filterNot(conflictPaths::contains).forEach { add(ChangedFile(it, ChangeKind.Added, staged = true)) }
            result.changed.filterNot(conflictPaths::contains).forEach { add(ChangedFile(it, ChangeKind.Modified, staged = true)) }
            result.removed.filterNot(conflictPaths::contains).forEach { add(ChangedFile(it, ChangeKind.Deleted, staged = true)) }
        }.sortedBy { it.path.lowercase() }
        val unstaged = buildList {
            result.modified.filterNot(conflictPaths::contains).forEach {
                add(ChangedFile(it, ChangeKind.Modified, staged = false))
            }
            result.missing.filterNot(conflictPaths::contains).forEach {
                add(ChangedFile(it, ChangeKind.Deleted, staged = false))
            }
            result.untracked.filterNot(conflictPaths::contains).forEach {
                add(ChangedFile(it, ChangeKind.Untracked, staged = false))
            }
        }.sortedBy { it.path.lowercase() }
        return RepositoryStatus(staged, unstaged, conflicts)
    }

    private fun branches(repository: Repository): List<BranchInfo> {
        val current = currentBranch(repository)
        return Git.wrap(repository).branchList().setListMode(ListBranchCommand.ListMode.ALL).call().map { ref ->
            val remote = ref.name.startsWith(Constants.R_REMOTES)
            val shortName = Repository.shortenRefName(ref.name)
            val localName = shortName.removePrefix("origin/")
            val tracking = if (!remote) runCatching { BranchTrackingStatus.of(repository, localName) }.getOrNull() else null
            val upstream = if (!remote) {
                val merge = repository.config.getString("branch", localName, "merge")
                val remoteName = repository.config.getString("branch", localName, "remote")
                merge?.let { "${remoteName ?: "origin"}/${Repository.shortenRefName(it)}" }
            } else null
            BranchInfo(
                name = shortName,
                isCurrent = !remote && localName == current,
                isRemote = remote,
                upstream = upstream,
                ahead = tracking?.aheadCount ?: 0,
                behind = tracking?.behindCount ?: 0,
            )
        }.sortedWith(compareByDescending<BranchInfo> { it.isCurrent }.thenBy { it.isRemote }.thenBy { it.name.lowercase() })
    }

    private fun currentBranch(repository: Repository): String = runCatching { repository.branch }
        .getOrElse { repository.fullBranch?.let(Repository::shortenRefName) ?: "HEAD" }

    private fun identity(repository: Repository): AuthorIdentity? {
        val name = repository.config.getString("user", null, "name")?.trim().orEmpty()
        val email = repository.config.getString("user", null, "email")?.trim().orEmpty()
        return if (name.isNotEmpty() && email.isNotEmpty()) AuthorIdentity(name, email) else null
    }

    private fun refsByCommit(repository: Repository): Map<String, List<String>> {
        val result = linkedMapOf<String, MutableList<String>>()
        val refs = repository.refDatabase.getRefsByPrefix(Constants.R_REFS)
        refs.forEach { ref ->
            val peeled = runCatching { repository.refDatabase.peel(ref) }.getOrDefault(ref)
            val id = peeled.peeledObjectId ?: ref.objectId ?: return@forEach
            val commitId = runCatching {
                RevWalk(repository).use { it.parseCommit(id).id.name() }
            }.getOrNull() ?: return@forEach
            val label = Repository.shortenRefName(ref.name)
            if (label != "origin/HEAD") result.getOrPut(commitId) { mutableListOf() }.add(label)
        }
        return result
    }

    private fun scanCommitDiff(
        repository: Repository,
        walk: RevWalk,
        commit: RevCommit,
        path: String? = null,
    ): List<DiffEntry> {
        val oldTree = if (commit.parentCount == 0) {
            EmptyTreeIterator()
        } else {
            val parent = walk.parseCommit(commit.getParent(0).id)
            treeParser(repository, parent.tree.id)
        }
        val newTree = treeParser(repository, commit.tree.id)
        return DiffFormatter(DisabledOutputStream.INSTANCE).use { formatter ->
            formatter.setRepository(repository)
            formatter.setDetectRenames(true)
            if (path != null) formatter.pathFilter = PathFilter.create(path)
            formatter.scan(oldTree, newTree)
        }
    }

    private fun formatDiff(repository: Repository, entry: DiffEntry): FileDiff {
        val output = LimitedByteArrayOutputStream(MAX_DIFF_BYTES)
        var binary = false
        DiffFormatter(output).use { formatter ->
            formatter.setRepository(repository)
            formatter.setDetectRenames(true)
            formatter.setContext(3)
            formatter.format(entry)
            binary = formatter.toFileHeader(entry).patchType.name == "BINARY"
        }
        val bytes = output.toByteArray()
        if (binary || RawText.isBinary(bytes)) {
            return FileDiff(displayPath(entry), entry.oldPath.takeUnless { it == DiffEntry.DEV_NULL }, emptyList(), output.truncated, true)
        }
        val text = decodeText(bytes)
        val parsed = parseUnifiedDiff(text)
        return FileDiff(
            path = displayPath(entry),
            oldPath = entry.oldPath.takeUnless { it == DiffEntry.DEV_NULL || it == entry.newPath },
            lines = parsed.take(MAX_DIFF_LINES),
            truncated = output.truncated || parsed.size > MAX_DIFF_LINES,
            binary = false,
        )
    }

    private fun parseUnifiedDiff(text: String): List<DiffLine> {
        var oldLine = 0
        var newLine = 0
        var inHunk = false
        val hunkRegex = Regex("^@@ -(\\d+)(?:,\\d+)? \\+(\\d+)(?:,\\d+)? @@")
        return buildList {
            text.lineSequence().forEach { line ->
                val hunk = hunkRegex.find(line)
                when {
                    hunk != null -> {
                        oldLine = hunk.groupValues[1].toInt()
                        newLine = hunk.groupValues[2].toInt()
                        inHunk = true
                        add(DiffLine(DiffLineKind.Header, line, null, null))
                    }

                    !inHunk || line.startsWith("diff --git") || line.startsWith("index ") ||
                        line.startsWith("--- ") || line.startsWith("+++ ") || line.startsWith("\\ No newline") -> {
                        add(DiffLine(DiffLineKind.Header, line, null, null))
                    }

                    line.startsWith("+") -> {
                        add(DiffLine(DiffLineKind.Added, line.drop(1), null, newLine))
                        newLine++
                    }

                    line.startsWith("-") -> {
                        add(DiffLine(DiffLineKind.Deleted, line.drop(1), oldLine, null))
                        oldLine++
                    }

                    else -> {
                        add(DiffLine(DiffLineKind.Context, line.removePrefix(" "), oldLine, newLine))
                        oldLine++
                        newLine++
                    }
                }
            }
        }
    }

    private fun treeParser(repository: Repository, treeId: ObjectId): CanonicalTreeParser {
        return CanonicalTreeParser().apply {
            repository.newObjectReader().use { reset(it, treeId) }
        }
    }

    private fun headTreeParser(repository: Repository): AbstractTreeIterator {
        val head = repository.resolve("HEAD^{tree}") ?: return EmptyTreeIterator()
        return treeParser(repository, head)
    }

    private fun RevCommit.toModel(refs: List<String>) = CommitInfo(
        sha = id.name(),
        shortSha = id.abbreviate(7).name(),
        message = shortMessage.ifBlank { "(no commit message)" },
        authorName = authorIdent.name.orEmpty().ifBlank { "Unknown author" },
        authorEmail = authorIdent.emailAddress.orEmpty(),
        authoredAt = authorIdent.whenAsInstant,
        parentShas = parents.map { it.id.name() },
        refs = refs,
    )

    private fun DiffEntry.ChangeType.toKind(): ChangeKind = when (this) {
        DiffEntry.ChangeType.ADD -> ChangeKind.Added
        DiffEntry.ChangeType.MODIFY -> ChangeKind.Modified
        DiffEntry.ChangeType.DELETE -> ChangeKind.Deleted
        DiffEntry.ChangeType.RENAME -> ChangeKind.Renamed
        DiffEntry.ChangeType.COPY -> ChangeKind.Copied
    }

    private fun displayPath(entry: DiffEntry): String =
        if (entry.newPath == DiffEntry.DEV_NULL) entry.oldPath else entry.newPath

    private fun requireObjectId(repository: Repository, revision: String): ObjectId =
        repository.resolve(revision) ?: throw GitOperationException("Commit '$revision' was not found.")

    private fun credentials(username: String, token: String): CredentialsProvider? =
        token.takeIf(String::isNotBlank)?.let {
            UsernamePasswordCredentialsProvider(username.ifBlank { "git" }, it)
        }

    private fun safeRepositoryName(value: String): String {
        val result = value.trim().removeSuffix(".git").replace(Regex("[^A-Za-z0-9._-]"), "-").trim('-', '.')
        require(result.isNotEmpty()) { "Repository name is invalid." }
        return result.take(80)
    }

    private fun nameFromUrl(url: String): String = url.trimEnd('/', '\\').substringAfterLast('/').substringAfterLast(':')

    private fun resolveInside(root: File, relativePath: String): File {
        val resolved = File(root, relativePath).canonicalFile
        require(resolved.path == root.path || resolved.path.startsWith(root.path + File.separator)) {
            "Path escapes the repository workspace."
        }
        return resolved
    }

    private fun deleteInside(root: File, relativePath: String) {
        val target = resolveInside(root, relativePath)
        if (!target.exists()) return
        require(target.path != root.path) { "Repository root cannot be deleted." }
        require(target.deleteRecursively()) { "Could not remove '$relativePath'." }
    }

    private fun decodeText(bytes: ByteArray): String = Charsets.UTF_8.newDecoder()
        .onMalformedInput(CodingErrorAction.REPLACE)
        .onUnmappableCharacter(CodingErrorAction.REPLACE)
        .decode(java.nio.ByteBuffer.wrap(bytes))
        .toString()

    private fun InputStream.readUpTo(limit: Int): ByteArray {
        val output = ByteArrayOutputStream(min(limit, DEFAULT_BUFFER_SIZE))
        val buffer = ByteArray(DEFAULT_BUFFER_SIZE)
        var remaining = limit
        while (remaining > 0) {
            val read = read(buffer, 0, min(buffer.size, remaining))
            if (read < 0) break
            output.write(buffer, 0, read)
            remaining -= read
        }
        return output.toByteArray()
    }

    private fun cleanMessage(error: Throwable, secret: String = ""): String {
        val fallback = error.javaClass.simpleName.replace(Regex("([a-z])([A-Z])"), "$1 $2")
        val raw = error.message?.takeIf { it.isNotBlank() } ?: fallback
        return if (secret.isNotBlank()) raw.replace(secret, "***") else raw
    }

    private suspend fun <T> io(block: () -> T): T = withContext(Dispatchers.IO) {
        try {
            block()
        } catch (error: GitOperationException) {
            throw error
        } catch (error: Throwable) {
            throw GitOperationException(cleanMessage(error), error)
        }
    }

    private class CallbackProgressMonitor(
        private val callback: (CloneProgress) -> Unit,
    ) : ProgressMonitor {
        private var task = "Preparing repository"
        private var total = ProgressMonitor.UNKNOWN
        private var completed = 0

        override fun start(totalTasks: Int) = Unit

        override fun beginTask(title: String, totalWork: Int) {
            task = title
            total = totalWork
            completed = 0
            callback(CloneProgress(task, completed, total))
        }

        override fun update(completed: Int) {
            this.completed += completed
            callback(CloneProgress(task, this.completed, total))
        }

        override fun endTask() {
            callback(CloneProgress(task, if (total > 0) total else completed, total))
        }

        override fun isCancelled(): Boolean = false
        override fun showDuration(enabled: Boolean) = Unit
    }

    private class LimitedByteArrayOutputStream(private val maxBytes: Int) : ByteArrayOutputStream() {
        var truncated: Boolean = false
            private set

        override fun write(value: Int) {
            if (count >= maxBytes) {
                truncated = true
                return
            }
            super.write(value)
        }

        override fun write(buffer: ByteArray, offset: Int, length: Int) {
            val remaining = maxBytes - count
            if (remaining <= 0) {
                truncated = true
                return
            }
            val accepted = min(length, remaining)
            super.write(buffer, offset, accepted)
            if (accepted < length) {
                truncated = true
            }
        }
    }

    private companion object {
        const val MAX_COMMITS = 5_000
        const val MAX_DIFF_BYTES = 512 * 1024
        const val MAX_DIFF_LINES = 4_000
        const val MAX_FILE_BYTES = 768 * 1024
    }
}

class GitOperationException(message: String, cause: Throwable? = null) : RuntimeException(message, cause)
