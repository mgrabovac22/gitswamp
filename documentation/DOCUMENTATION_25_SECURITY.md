# Security Considerations

## 1. Authentication & Authorization

### 1.1 Local Repository Access

**Current Implementation:**
- No built-in authentication for local repositories
- Relies on file system permissions
- Full access to opened repositories

**Recommendations:**
- Validate file path before opening
- Check write permissions before operations
- Display warnings for sensitive operations

### 1.2 Remote Repository Access

**Supported Methods:**
- HTTPS with credentials
- SSH with key-based authentication
- Personal access tokens for provider APIs (GitHub, GitLab, Bitbucket, Azure DevOps)

**Implementation:**
```rust
// Credential caching in git2-rs
let cred_helper = git2::CredentialHelper::new("origin");
let cred = cred_helper.resolve(&url, None, &capabilities)?;
```

## 2. Credential Management

### 2.1 Token Storage

**Security Measures:**
- Tokens are stored locally per provider key with lightweight obfuscation
- Protected by file system permissions
- Never logged or displayed
- Loaded only when a provider integration needs them

**Important:** The current implementation uses base64 plus an application XOR key for obfuscation, not OS keychain storage and not cryptographic encryption. Treat local machine access as trusted and use scoped, revocable provider tokens.

**Storage Locations:**
- Windows: `%APPDATA%\.gitswamp\credentials_<provider>`
- macOS/Linux: `$HOME/.config/.gitswamp/credentials_<provider>`

**Provider Keys:**
- `github`
- `github-enterprise`
- `gitlab`
- `gitlab-self`
- `bitbucket`
- `bitbucket-dc`
- `azure`
- `azure-domain`

Self-hosted GitLab token data is stored as `domain|token` so the frontend can match the token to the configured instance.

### 2.2 GitHub Integration

**Authentication Methods:**
- Personal access token
- GitHub Enterprise token when a custom domain is configured

**Recommended Scopes:**
```
repo - Repository access
gist - Gist access
read:user - User profile
```

### 2.3 GitLab Integration

**Authentication Methods:**
- Private access token
- Personal access token with api scope
- Self-managed GitLab token stored with normalized domain

**Recommended Scopes:**
```
api - Full API access
read_user - Read user profile
read_repository - Read repository
```

### 2.4 Other Provider Integration

**Bitbucket:**
- Uses app passwords or HTTP access tokens depending on provider mode
- Store only repository-scoped credentials where possible

**Azure DevOps:**
- Uses personal access tokens
- Stores the Azure organization/domain separately under `azure-domain`

**Recommendation:** Prefer the smallest scopes that support clone/search/push workflows, rotate tokens periodically, and revoke unused tokens from the provider UI.

## 3. Input Validation

### 3.1 Path Validation

**Rules:**
- Must be absolute path
- Cannot contain `../` sequences
- Must exist on file system
- User must have read permissions

**Implementation:**
```rust
fn validate_repository_path(path: &str) -> Result<(), String> {
    if path.contains("..") {
        return Err("Invalid path: contains parent directory references".into());
    }

    let path = std::path::Path::new(path);
    if !path.exists() {
        return Err("Repository path does not exist".into());
    }

    if !path.is_dir() {
        return Err("Path is not a directory".into());
    }

    Ok(())
}
```

### 3.2 Branch Name Validation

**Git Branch Naming Rules:**
- Cannot start with `-`
- Cannot contain `~^:?*[\\` characters
- Cannot contain `//` or end with `/`
- Cannot end with `.lock`
- Must not be empty

**Implementation:**
```rust
fn validate_branch_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Branch name cannot be empty".into());
    }

    if name.starts_with('-') {
        return Err("Branch name cannot start with '-'".into());
    }

    if name.contains("..") || name.contains("//") {
        return Err("Invalid branch name pattern".into());
    }

    Ok(())
}
```

### 3.3 Commit Message Validation

**Rules:**
- Must not be empty
- Max length: 72 characters (first line)
- Max total: 1000 characters
- Should follow conventional commits

### 3.4 URL Validation

**Validation:**
- Valid Git remote URL format
- URL scheme validation (https://, git://, ssh://)
- No command injection

```rust
fn validate_repository_url(url: &str) -> Result<(), String> {
    if url.is_empty() {
        return Err("URL cannot be empty".into());
    }

    // Allow common Git URL formats
    if !url.starts_with("https://") &&
       !url.starts_with("git://") &&
       !url.starts_with("ssh://") &&
       !url.contains('@') {
        return Err("Invalid repository URL".into());
    }

    Ok(())
}
```

## 4. File Access Security

### 4.1 File Operations

**Restrictions:**
- Only access files within repository working directory
- Cannot write outside repository
- Validate all file paths

**Implementation:**
```rust
fn is_file_in_repository(repo_path: &str, file_path: &str) -> Result<bool, String> {
    let repo = std::path::PathBuf::from(repo_path);
    let file = std::path::PathBuf::from(file_path);

    // Check if file is within repository
    file.starts_with(&repo)
        .then_some(true)
        .ok_or_else(|| "File outside repository".into())
}
```

### 4.2 Configuration Access

**Protected:** Git user configuration files

**Allowed Operations:**
- Read repository configuration
- Read user name/email
- Write user name/email

**Disallowed Operations:**
- Access global Git config
- Modify hooks
- Access SSH keys

## 5. IPC Security

### 5.1 Command Validation

**Frontend → Backend:**
- Validate command parameters
- Type checking via serde
- Size limits on inputs
- Timeout on long operations

### 5.2 Response Sanitization

**Backend → Frontend:**
- Serialize only necessary data
- No sensitive information in responses
- Error messages don't leak paths

## 6. Network Security

### 6.1 API Communication

**HTTPS Only:**
```rust
// Force HTTPS for GitHub API
let client = ureq::Agent::new();
let response = client
    .get("https://api.github.com/repos/search")  // Always HTTPS
    .call()?;
```

### 6.2 Certificate Validation

**Enabled by Default:**
- SSL/TLS certificate verification
- No insecure certificate acceptance
- Modern cipher suites

### 6.3 Rate Limiting

**GitHub API:**
- 60 requests/hour (unauthenticated)
- 5000 requests/hour (authenticated)
- Implementation: Backoff on 429 responses

**GitLab API:**
- 300 requests/10 minutes (unauthenticated)
- 600 requests/10 minutes (authenticated)

## 7. Data Protection

### 7.1 In-Memory Data

**Sensitive Data Clearing:**
```rust
// Clear credentials after use
impl Drop for Credential {
    fn drop(&mut self) {
        // Overwrite memory before dropping
        self.token.zeroize();
    }
}

use zeroize::Zeroize;
let mut sensitive = String::from("secret");
sensitive.zeroize();  // Securely clear
```

### 7.2 File Permissions

**Configuration Files:**
- Mode 600 (owner read/write only)
- No world-readable tokens

**Implementation:**
```rust
use std::fs;
use std::os::unix::fs::PermissionsExt;

fs::write(&path, content)?;
fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
```

## 8. Logging Security

### 8.1 What to Log

**Safe to Log:**
- Operation names (clone, commit, push)
- Repository paths (anonymized)
- Success/failure status
- Execution time

**Never Log:**
- Credentials or tokens
- Full error messages with paths
- User input content
- Sensitive commit messages

### 8.2 Log Levels

```rust
// Debug: Operation details (development only)
debug!("Cloning repository from {}", repo_url);

// Info: High-level operations
info!("Repository successfully opened");

// Error: Operation failures
error!("Failed to stage file: {}", error_code);
```

## 9. Dependency Security

### 9.1 Rust Dependencies

**Regular Updates:**
```bash
# Check for vulnerable dependencies
cargo audit

# Update dependencies
cargo update

# Lock file usage
# Cargo.lock included in repository
```

**Key Dependencies Review:**
- git2 - Maintained by GitLab/libgit2
- serde - Standard serialization
- tokio - Maintained async runtime
- ureq - Lightweight HTTP

### 9.2 JavaScript Dependencies

**Regular Updates:**
```bash
# Check for vulnerable packages
npm audit

# Update to latest
npm update

# Lock file usage
# package-lock.json included
```

**Key Dependencies Review:**
- Vue 3 - Maintained by Vue team
- Tauri - Maintained by Tauri team

## 10. Transport Security

### 10.1 SSH Configuration

**Supported:**
- RSA keys (4096 bits minimum)
- ED25519 keys (preferred)
- ECDSA keys
- Key passphrases

**Not Supported:**
- DSA keys (deprecated)
- Insecure key sizes

### 10.2 HTTPS Configuration

**TLS Version:**
- Minimum: TLS 1.2
- Preferred: TLS 1.3

**Certificate Validation:**
- Always enabled
- No option to disable
- Strict hostname checking

## 11. Permission Model

### 11.1 File System Permissions

**Required for Open:**
- Read permission on repository directory
- Execute permission on path traversal

**Required for Operations:**
- Write permission for commits
- Write permission for stashing
- Write permission for branch operations

**Enforcement:**
```rust
// Check permissions before operation
fn check_write_permission(repo_path: &str) -> Result<(), String> {
    let metadata = std::fs::metadata(repo_path)
        .map_err(|e| e.to_string())?;

    if !metadata.permissions().readonly() {
        Ok(())
    } else {
        Err("Write permission denied".into())
    }
}
```

### 11.2 Git Configuration Permissions

**Readable:**
- User name
- User email
- Remote URLs
- Branch tracking

**Write Access Required:**
- Creating commits
- Creating branches
- Modifying configuration

## 12. Security Best Practices

### 12.1 For Users

1. **Keep GitSwamp Updated**
   - Security patches released regularly
   - Update promptly

2. **Protect SSH Keys**
   - Use strong passphrases
   - Restrict key permissions
   - Backup securely

3. **Token Security**
   - Use tokens with minimal scopes
   - Rotate tokens periodically
   - Revoke unused tokens

4. **Repository Access**
   - Only open trusted repositories
   - Review file changes before committing
   - Verify commit history

### 12.2 For Developers

1. **Code Review**
   - Review all path-related code
   - Validate user inputs
   - Test security scenarios

2. **Dependency Management**
   - Regular audits with `cargo audit` and `npm audit`
   - Update critical security fixes immediately
   - Review changelog for security issues

3. **Testing**
   - Test with malicious inputs
   - Fuzzing for path validation
   - Security-focused test cases

## 13. Incident Response

### 13.1 Security Issues

**Reporting:**
- Report to security@wortex-tech.com
- Do not disclose publicly
- Allow 90 days for fix

### 13.2 Vulnerability Disclosure

**Timeline:**
- Day 0: Report received
- Day 30: Initial assessment
- Day 60: Patch development
- Day 90: Public disclosure

## 14. Compliance

### 14.1 GDPR Compliance

**Data Handling:**
- No user data collection
- No analytics tracking
- Local-only operation
- User full control

### 14.2 Security Standards

**Follows:**
- OWASP Top 10 guidelines
- CWE (Common Weakness Enumeration)
- Git security best practices

---

**Related Documentation:**
- [25_SECURITY.md](./DOCUMENTATION_25_SECURITY.md) - Extended security documentation
- [27_LOGGING_MONITORING.md](./DOCUMENTATION_27_LOGGING_MONITORING.md) - Logging practices
