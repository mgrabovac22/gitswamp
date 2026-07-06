# User Guide

## 1. Getting Started

### 1.1 Installation

**Download GitSwamp:**
1. Visit [GitHub Releases](https://github.com/mgrabovac22/gitswamp/releases)
2. Download for your operating system:
   - Windows: `gitswamp-setup.msi`
   - macOS: `GitSwamp.dmg`
   - Linux: `gitswamp.AppImage`

**Install:**

**Windows:**
- Run the `.msi` installer
- Follow installation wizard
- Application starts automatically

**macOS:**
- Open `.dmg` file
- Drag GitSwamp to Applications folder
- Launch from Applications

**Linux:**
- Make AppImage executable: `chmod +x gitswamp.AppImage`
- Run: `./gitswamp.AppImage`

### 1.2 First Launch

1. Open GitSwamp
2. Choose action:
   - **Open Repository** - Load existing Git repository
   - **Clone Repository** - Clone from GitHub/GitLab
   - **Initialize Repository** - Create new repository
3. Use the start dashboard:
   - If a GitHub token is configured, review pull requests authored by you and issues assigned to you
   - If no token is configured, use the local repository summary and recent repositories
   - Use Settings when you want to connect provider tokens

## 2. Opening a Repository

### 2.1 Open Existing Repository

1. Click "Open Repository"
2. Select folder containing `.git` directory
3. Click "Open"
4. Application loads repository

### 2.2 Clone Repository

1. Click "Clone Repository"
2. Enter repository URL:
   - HTTPS: `https://github.com/user/repo.git`
   - SSH: `git@github.com:user/repo.git`
3. Select destination folder
4. Click "Clone"
5. Application opens cloned repository

### 2.3 Initialize New Repository

1. Click "Initialize Repository"
2. Select empty folder
3. Click "Initialize"
4. New repository created and opened

## 3. Understanding the Interface

### 3.1 Main Areas

**Top Bar (Title Bar):**
- App title and window controls (minimize, maximize, close)

**Header:**
- Current branch name
- Commit count
- Remote status

**Repository Tabs + Quick Menu:**
- Multi-tab repository strip
- Hamburger menu with 4 sections: File, Edit, View, Help
- New tab button
- Close current tab button

**Left Sidebar:**
- Branches section (expand/collapse)
- Stashes section
- Tags section

**Main Area:**
- Commit graph or selected history/analytics view (center)
- Commit details (right panel)

**Bottom:**
- Status bar
- Integrated terminal output (if visible)

**Start Dashboard:**
- Shows GitHub work items when authenticated
- Shows local repository context and recent repositories when offline or unauthenticated
- Provides search for pull requests and assigned issues

### 3.2 Quick Menu (Hamburger ☰)

Click the top-left hamburger icon to open the quick menu with 4 sections:

**File:**
- **New Tab** (Ctrl+T) - Open fresh repository tab
- **Open Repository** (Ctrl+O) - Browse and open a Git repository
- **Close Current Tab** (Ctrl+W) - Close currently selected tab
- **Create a Gist** (Ctrl+Shift+G) - Launch GitHub Gist creator

**Edit:**
- **Copy Repository Path** (Ctrl+Shift+C) - Copy active folder path to clipboard
- **Refresh Repository** (Ctrl+Shift+R) - Reload commits, branches, status, and tags
- **Open in VS Code** (Ctrl+Shift+O) - Launch active repository in VS Code

**View:**
- **Toggle Terminal** (Ctrl+`) - Show or hide integrated git terminal
- **Visualize Commit History** (Alt+1) - Switch to Graph view (commit history visualization)
- **Galaxy View** (Alt+2) - Explore commit history as an interactive canvas galaxy/tree
- **Productivity Arena** (Alt+3) - View developer productivity metrics and activity
- **Time Machine** (Alt+4) - Navigate through repository history frames
- **Usual Conflict Suspects** (Alt+5) - View merge conflict hotspots (heatmap)
- **Burndown Analytics** (Alt+6) - View team focus, after-hours, and repository pulse signals
- **Open Folder Explorer** (Alt+O) - Open repository folder in system explorer
- **Open Settings** (Ctrl+,) - Open app settings dialog

**Help:**
- **Features and Shortcuts** (F1) - View in-app help panel with all shortcuts
- **Online Guide** - Open full documentation in browser
- **Report Issue** - Navigate to GitHub issue tracker

Hover over any menu section to see available actions; click to execute.

## 4. Viewing Commits

### 4.1 Commit Graph

The main area shows your commit history as a graph:
- **X-axis:** Horizontal timeline
- **Lanes:** Vertical branches
- **Lines:** Parent-child relationships
- **Dots:** Individual commits

**Interaction:**
- Click commit to select it
- Double-click to checkout
- Right-click for context menu

### 4.2 Commit Details

Right panel shows selected commit:
- **Header:** Commit hash (copyable)
- **Author:** Name and email
- **Date:** Commit timestamp
- **Message:** Full commit message
- **Files:** List of changed files
- **Stats:** Total insertions/deletions

### 4.3 History View Modes

GitSwamp provides multiple ways to visualize your repository history. Switch between them using the View menu or keyboard shortcuts:

#### Graph View (Alt+1) - Default
The interactive commit history graph showing:
- Branch structure and merges
- Commit timeline
- All commits in chronological order
- Interactive commit selection and navigation

#### Galaxy View (Alt+2)
Explore history on a canvas-based map:
- Spiral, layered, and constellation layouts
- 3D, balanced, and circling motion modes
- Tree mode for branch hierarchy visualization
- Zoom, pan, click-to-focus, and hover details
- Useful for understanding how branches and commits relate visually

#### Productivity Arena (Alt+3)
Visualize developer activity and metrics:
- Start in preview mode, then use **Load all** for the full commit history
- Filter metrics by author to compare one contributor against the whole team
- Commit frequency by contributor
- Time-based activity patterns, streaks, and weekend/off-hours activity
- File change statistics and deletion intensity
- Team productivity comparison, balance score, and arena health score
- Stability indicators derived from conflict pressure and regression-like commit messages
- Useful for performance analysis and sprint planning

#### Time Machine (Alt+4)
Navigate through repository history frame-by-frame:
- View repository state at specific commits
- Move backward and forward one frame at a time
- Enable autoplay or reverse autoplay for history playback
- Search commits by SHA and jump through matches
- Browse the repository tree snapshot and preview file contents at that commit
- Copy a rollback command for the selected commit or selected file
- Compare file versions across time
- Inspect legacy code without checking out
- Smooth autoplay and arrow-step transitions (reduced visual flicker)
- Snapshot panels keep context and update progressively with section loaders and caches
- Useful for forensic analysis and understanding repository evolution

#### Usual Conflict Suspects (Alt+5)
See merge conflict hotspots and risky areas:
- Files with highest conflict frequency
- Conflict risk assessment
- Team collaboration conflict patterns
- Repository tree view with folder collapse/expand
- File-level risk coloring and directory risk roll-up based on child hotspots
- Conflict pair diagnostics (which file pairs often collide together)
- Merge-window selector for 300, 500, 1000, or all history
- Search filtering and tree-scope filtering for focused investigation
- Helps prevent conflicting changes

#### Burndown Analytics (Alt+6)
Understand team focus and repository pulse:
- Recent weekly repository activity
- After-hours and weekend indicators
- Contributor workload and focus-risk signals
- Hot-file and bottleneck context
- Uses readable labels and lightweight loading so scrolling stays responsive

## 5. Managing Branches

### 5.1 Switch Branches

**Method 1: Sidebar**
1. Find branch in "Branches" section
2. Click to checkout
3. Working directory updates

**Method 2: Header**
1. Click branch name in header
2. Select branch from dropdown
3. Click to switch

### 5.2 Create New Branch

1. Click "+" next to "Branches" in sidebar
2. Enter new branch name
3. Select source (current: HEAD)
4. Click "Create"

**Branch Naming Tips:**
- Use lowercase with hyphens
- Examples: `feature/new-ui`, `fix/memory-leak`

### 5.3 Delete Branch

1. Right-click branch in sidebar
2. Select "Delete Branch"
3. Confirm deletion
4. Branch removed

### 5.4 Rename Branch

1. Right-click branch in sidebar
2. Select "Rename Branch"
3. Enter new name
4. Confirm

## 6. Staging and Committing

### 6.1 View Changes

1. Open repository
2. Left sidebar shows changed files
3. Click file to view diff in right panel

### 6.2 Stage Files

**Stage Single File:**
1. Click file in changes list
2. Click "Stage" button
3. File moves to "Staged" section

**Stage All Files:**
1. Click "Stage All" button
2. All changes staged

**Stage a Hunk:**
1. Open a changed file in the diff viewer
2. Find the hunk header
3. Click "Stage hunk"
4. Only that hunk moves into the staged version

**Unstage a Hunk:**
1. Open a staged diff
2. Click "Unstage hunk"
3. Only that hunk returns to unstaged changes

### 6.3 View Diffs

**Working Directory Diff:**
1. Click unstaged file
2. Right panel shows diff
3. Toggle between:
   - Side-by-side view
   - Unified view

**Commit Diff:**
1. Click commit in history
2. Click file in file list
3. Shows changes in that commit

**File Journey:**
1. Open a file diff
2. Click the small File Journey icon in the diff toolbar
3. Review quick created-by and last-changed context
4. Close the small panel with its X button

**Smart .gitignore Assistant:**
1. Leave generated/private files untracked
2. If GitSwamp detects common patterns such as `node_modules/`, `dist/`, `target/`, `.env`, or `*.log`, the assistant appears in the changes panel
3. Choose **Ignore selected**, **Open .gitignore**, or **Keep tracking**
4. Existing `.gitignore` rules are respected and duplicate patterns are skipped

### 6.4 Create Commit

1. Stage desired files
2. Scroll down to "Commit Message"
3. Enter message (first line < 50 chars)
4. Add detailed description (optional)
5. Click "Commit"
6. New commit appears in graph

**Visual Commit Builder:**
1. Click the builder icon beside the commit analyzer/message area
2. Pick a type, scope, optional issue number, and summary
3. Click Save to place the generated subject into the commit input

**Commit Message Format:**
```
Feat(frontend): #123 add new feature

This is the detailed description.
It can span multiple lines.
Each line should be under 72 characters.
```

## 7. Pushing and Pulling

### 7.1 Push Commits

1. Create commits locally
2. Click "Push" in header
3. Commits sent to remote
4. Remote branch updates

### 7.2 Pull Changes

1. Click "Pull" in header
2. Fetches and merges from remote
3. Local branch updates
4. Commit graph refreshes
5. After a successful merge flow, GitSwamp may offer to download generated release notes

### 7.3 Fetch Changes

1. Click "Fetch" button
2. Downloads latest remote info
3. Prunes deleted remote branches when the fetch flow is configured to do so
4. No merge (safe operation)
5. Check differences before pulling

## 8. Working with Stashes

### 8.1 Stash Changes

1. Make changes (don't stage)
2. Click "Stash" button
3. Enter stash message
4. Working directory cleaned
5. Stash appears in sidebar

### 8.2 Apply Stash

1. Find stash in sidebar
2. Click stash
3. Click "Apply" button
4. Changes restored to working directory

### 8.3 Pop Stash

1. Find stash in sidebar
2. Click stash
3. Click "Pop" button
4. Changes restored
5. Stash deleted

## 9. Tags

### 9.1 View Tags

1. Expand "Tags" section in sidebar
2. See all repository tags
3. Click tag to jump to commit

### 9.2 Create Tag

1. Click commit in history
2. Right-click, select "Create Tag"
3. Enter tag name
4. Choose lightweight or annotated
5. Tag created

### 9.3 Delete Tag

1. Right-click tag in sidebar
2. Select "Delete Tag"
3. Confirm
4. Tag removed

## 10. Settings and Preferences

### 10.1 Open Settings

1. Click gear icon (top right)
2. Settings dialog opens

### 10.2 Theme

1. Open Settings
2. Toggle "Dark Mode" / "Light Mode"
3. Interface updates immediately

### 10.3 Font Size

1. Open Settings
2. Adjust "Font Size" slider
3. Preview updates in real-time
4. Apply to all text

### 10.4 Compact Mode

1. Open Settings
2. Toggle "Compact Mode"
3. UI reduces padding and margins
4. More content visible

### 10.5 Safety and Background Preferences

Settings also include optional workflow helpers:
- Smart .gitignore Wizard, enabled by default
- Reduced motion and graph animation preferences
- Background maintenance reminders, off by default
- Large change reminders with a configurable threshold
- Conflict, stale work, and behind-branch reminders shown as toasts
- Commit details preload, off by default for lower RAM usage

## 11. Searching Commits

### 11.1 Search by Message

1. Click search icon
2. Type commit message
3. Results filter in real-time
4. Click result to jump

### 11.2 Search by Author

1. Click search icon
2. Prefix with `author:`
3. Type author name
4. Results show matching commits

## 12. Resolving Conflicts

### 12.1 Merge Conflicts

If conflicts occur during pull/merge:

1. Dialog appears showing conflicted files
2. Conflict Resolver opens automatically
3. Show three versions:
   - **Original:** Base version
   - **Current:** Your version
   - **Incoming:** Remote version

### 12.2 Resolving

1. View conflicts side-by-side
2. Choose resolution:
   - "Use Ours" - Keep local changes
   - "Use Theirs" - Accept remote changes
   - "Use Both" - Combine both
3. Edit manually if needed
4. Mark resolved
5. Commit to complete merge

## 13. Keyboard Shortcuts

| Shortcut | Action | Menu Location |
|----------|--------|---------------|
| `F1` | Open in-app help and shortcuts panel | Help → Features and Shortcuts |
| `Ctrl+K` | Open command palette | Global |
| `Ctrl+T` | New repository tab | File → New Tab |
| `Ctrl+O` | Open repository | File → Open Repository |
| `Ctrl+W` | Close current tab | File → Close Current Tab |
| `Ctrl+Tab` | Switch to next repository tab | Global |
| `Ctrl+Shift+T` | Reopen last closed repository tab | File → Reopen Closed Tab |
| `Ctrl+,` | Open settings | View → Open Settings |
| `Ctrl+`` | Toggle terminal panel | View → Toggle Terminal |
| `Ctrl+Shift+C` | Copy repository path | Edit → Copy Repository Path |
| `Ctrl+Shift+R` | Refresh repository | Edit → Refresh Repository |
| `Ctrl+Shift+O` | Open in VS Code | Edit → Open in VS Code |
| `Ctrl+Shift+G` | Open Gist creator | File → Create a Gist |
| `Ctrl+Shift+I` | Open integrations/options | View/Options |
| `Ctrl+Shift+K` | Open settings/preferences | View/Options |
| `Ctrl+Shift+L` | Toggle logs | Help/Debug |
| `Alt+O` | Open in folder explorer | View → Open Folder Explorer |
| `Alt+1` | Switch to Graph view (Commit history) | View → Visualize Commit History |
| `Alt+2` | Switch to Galaxy View | View → Galaxy View |
| `Alt+3` | Switch to Productivity Arena | View → Productivity Arena |
| `Alt+4` | Switch to Time Machine | View → Time Machine |
| `Alt+5` | Switch to Conflict Heatmap | View → Usual Conflict Suspects |
| `Alt+6` | Switch to Burndown Analytics | View → Burndown Analytics |
| `Ctrl+R` | Focus commit search | (Global) |

**Note:** `Ctrl+`` means Ctrl + Backtick (key below Esc on most layouts).

## 14. Tips and Tricks

### 14.1 Hamburger Menu Navigation

- Hover over menu section tabs (File, Edit, View, Help) to explore all actions
- Each section highlights on hover with contextual actions on the right
- Click any action or press keyboard shortcut shown in-menu
- Press `Esc` to close menu at any time

### 14.2 Opening Repositories

**Quick access:**
- **Alt+O** - Open current repository in system folder explorer (Windows Explorer, Finder, Nautilus, etc.)
- **Ctrl+Shift+O** - Open current repository in VS Code
- **Ctrl+T** - Open new repository tab
- **Ctrl+O** - Browse and open different repository

### 14.3 Keyboard-First Workflow

- Most menu actions have keyboard shortcuts displayed in-menu
- Combine with global shortcuts for efficient workflow:
  - **Ctrl+T** → (work in new tab) → **Ctrl+W** (close when done)
  - **Ctrl+Tab** → move through open repositories
  - **Ctrl+K** → search actions directly
  - **Ctrl+Shift+R** → Refresh → **Alt+O** → Browse files in explorer

### 14.4 In-App Help

- Press **F1** anytime to open help panel
- View:
  - Complete feature list
  - All keyboard shortcuts with descriptions
  - Quick start tips
- Panel closes with `Esc` or by clicking the X button
1. Make changes
2. Stage files
3. Type message
4. Press `Ctrl+Enter` to commit

**Feature Branch Workflow:**
1. Create branch from main
2. Make commits
3. Push branch
4. Create pull request on GitHub

### 14.5 Viewing History

**See who changed what:**
1. Click file in commit
2. View file diff
3. Shows exact changes by line

**Track branch evolution:**
1. Right-click branch
2. View branch history
3. See commits on this branch

### 14.6 Safe Operations

**Before dangerous operations:**
1. Create backup branch
2. Stash uncommitted changes
3. Verify you're on correct branch
4. Review changes before committing

**GitSwamp safety helpers:**
- UI destructive actions such as discard run after a short undo toast window
- The integrated terminal previews destructive commands such as `git reset --hard`, forced `git clean`, destructive restore, branch delete, stash pop/drop/clear, and force push
- When possible, the terminal safety prompt can create a safety stash before continuing
- Discard actions for changes are scoped to the intended unstaged working-tree changes, not staged changes

## 15. Troubleshooting

### 15.1 Application Won't Start

**Windows:**
- Run as Administrator
- Check Windows Defender allows it
- Reinstall latest version

**macOS:**
- Allow in Security & Privacy settings
- Install required dependencies

**Linux:**
- Ensure executable permission: `chmod +x gitswamp.AppImage`
- Install required libraries

### 15.2 Repository Won't Open

**Possible Causes:**
- Path doesn't contain `.git` folder
- Insufficient permissions
- Corrupted repository

**Solutions:**
- Verify path contains `.git`
- Check file permissions
- Run `git fsck` to check integrity

### 15.3 Folder Explorer Does Not Open

**Symptoms:**
- Clicking "Open in Folder Explorer" does nothing
- Alt+O fails with a path permission error

**Solutions:**
- Ensure a repository is open in the active tab
- Retry from the hamburger menu: View -> Open in Folder Explorer
- Update to the latest GitSwamp build where explorer opening uses backend tool integration

### 15.4 Can't Push/Pull

**Common Issues:**
- Authentication failed
- Network connection
- Remote deleted

**Solutions:**
- Verify git credentials
- Check internet connection
- Fetch to see remote status
- Delete and recreate remote if needed

### 15.5 Commits Not Showing

**Causes:**
- Filter applied
- Wrong branch selected
- Loading in progress

**Solutions:**
- Clear search filters
- Switch to correct branch
- Wait for commit loading
- Fetch from remote

## 16. Getting Help

### 16.1 Documentation

- Check [GitHub Wiki](https://github.com/mgrabovac22/gitswamp/wiki)
- See [FAQ Section](#faq)
- Review [Video Tutorials](#tutorials)

### 16.2 Report Issues

1. Check [existing issues](https://github.com/mgrabovac22/gitswamp/issues)
2. Provide:
   - OS and version
   - GitSwamp version
   - Steps to reproduce
   - Expected vs actual behavior
   - Screenshots if applicable

### 16.3 Feature Requests

1. Check [feature requests](https://github.com/mgrabovac22/gitswamp/discussions)
2. Describe use case
3. Explain expected behavior
4. Vote on similar requests

---

**For Technical Documentation:**
- Developers: See [Development Setup](./DOCUMENTATION_19_DEVELOPMENT_SETUP.md)
- Admins: See [Deployment Guide](./DOCUMENTATION_22_DEPLOYMENT_GUIDE.md)
