# API Pulse

A Postman-style HTTP/REST client that lives in the central view area of a repository tab. Requests are executed by the Rust backend (`ureq`), so cookies, redirects and TLS are handled outside the webview. Everything is persisted per repository and globally, mirrored to the application data folder and restored on the next start.

Open it from the view tabs, the command palette (`View: API Pulse`), or with `Alt+6`.

## Layout

- **Left rail** — request tabs/side list with collections, history and environments.
- **Center** — request editor (or run details / simulated tests when opened).
- **Right panel** — response viewer with body, headers, cookies, timeline and tests tabs.

## Collections and Requests

- **Scopes**: collections are either repository scoped (visible only while that repository is open) or global (shared across every repository).
- **Hierarchy**: collections can be nested up to eight levels; requests are sorted by `order` inside their folder.
- **Inline renaming**: click the pencil on a request row and edit the name in place; collections rename the same way.
- **Drag and drop**: drag a request row onto another collection header to move it, or onto the unfiled group to detach it.
- **Deletion safety**: deleting a collection opens a confirmation dialog before anything is removed.
- **Ordering**: move requests up/down, duplicate them, or export them individually.
- **Per-collection run summary**: after a run, each collection shows a compact `passed/total` chip with the average duration and a chevron that opens run details filtered to that collection.

## Requests

- Methods, URL with `{{variable}}` interpolation, JSON/text bodies and key/value headers.
- Advanced options expose repeat count (1–500) and delay in milliseconds (0–60000) for the request.
- The header-set button opens shared, reusable header documents that can be scoped globally or to a repository and imported/exported as JSON.
- The cookie manager is organized per domain and can clear all cookies for one domain.
- The key/value editor only commits the new draft row on `Enter` or when the whole row loses focus, so partial values are not turned into rows while you type.

## Header Precedence

Request own headers → collection chain (from the request folder up to its root) → shared header documents attached to the request. Importing a bundle remaps identifiers and preserves scopes so nothing is overwritten accidentally.

## Environments

- Environments hold variable sets (`{{name}}`); `Local` is the default environment and cannot be deleted.
- The active environment is shown in the Environments tab (not duplicated in the status bar).
- Session variables captured by extractors are listed in this tab as well.

## Runs

- `Run all` and the collection play button execute requests sequentially; the collection-level run runs only that folder.
- A run can start from the right panel or the command palette; results open in the center view (list of steps plus overview, assertions, headers and body tabs) instead of a modal.
- Only one run or request can be active at a time.
- Run steps and statistics (total, passed, failed, avg, p95) survive restarts.
- Opening run details from a collection chip filters the list to that collection only; the generic Details button shows every step.

## Simulated Tests

- Opened from the bottom of the right panel; the view has profile selection on the left, results in the middle and a detail pane on the right.
- Profiles: smoke, sanity, latency, load, spike, stress, soak and regression. Each selected profile sends one real probe request and simulates the remaining traffic with seeded jitter.
- **Editing a profile**: click a profile row to replace the RESULTS column with its editor. Basic settings (iterations, expected status) live in the center column and advanced settings (latency budget, i.e. the p95 threshold) live in the right column. Press **Save** to persist and return to RESULTS, or **Cancel**. Running tests always returns to RESULTS. Use **Reset to defaults** to discard custom values.
- Profile settings are stored with the rest of the client settings and restored on the next start.
- **Checkboxes** only toggle whether a profile is included in the next run; the row click opens the editor.
- Results default to the latest smoke run when the selected request has no results yet.
- Extracted values from extractors are listed at the bottom left of the view.

### Statistics

The detail pane shows everything obtained for the selected result:

- **Request**: method, name and resolved URL, plus the timestamp and profile.
- **Throughput**: iterations, request count, passed, failed, pass rate, error rate, requests/second and total simulated time.
- **Latency**: min, p50, average, p95, max and spread.
- **Baseline vs simulated**: baseline status, expected status, latency budget, verdict and the profile note.

#### What is p95?

The **95th percentile** latency: 95% of the simulated requests were at least as fast as this value, and only the slowest 5% were slower. It highlights worst-case behaviour far better than the average, which can hide slow outliers. Lower is better, and it should stay under the configured latency budget for the profile to pass.

## Response Viewer

- **Body** modes: pretty (syntax highlighted for JSON/XML/HTML/JS/CSS), raw and hex.
- Search inside the body; matches are highlighted. When a search query is active, plain (unhighlighted) rendering is used so matches are easy to spot.
- **Save** opens a format dialog and writes the body to disk through the native save dialog as **Original**, **JSON**, **HTML** or **Text**.
- **Headers**, **Cookies**, **Timeline** and **Tests** tabs round out the response.

## Export and Import

- JSON export of the whole workspace, a single collection or a single request through the save dialog.
- Import uses the open dialog and remaps identifiers, so repeated imports create new entries rather than overwriting existing ones.

## Shortcuts

| Shortcut | Action |
| --- | --- |
| `Alt+6` | Open APIPulse |
| `Enter` (URL bar) | Send the active request |
