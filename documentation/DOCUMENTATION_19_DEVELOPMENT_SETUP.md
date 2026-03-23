# Development Setup Guide

## 1. Prerequisites

### 1.1 System Requirements

#### Operating Systems
- Windows 10+ with Build Tools
- macOS 10.13+ with Xcode Command Line Tools
- Linux (Ubuntu 20.04+, Fedora 30+, etc.) with build-essential

#### Hardware Requirements
- CPU: Dual-core 2.0 GHz or higher
- RAM: 4 GB minimum, 8 GB recommended
- Disk: 1 GB free space for tools and dependencies

### 1.2 Required Software

#### Node.js & npm
```bash
# Download from https://nodejs.org/
# LTS version recommended (18.x or higher)
node --version  # Should show v18.x.x or higher
npm --version   # Should show 9.x.x or higher
```

#### Rust & Cargo
```bash
# Install from https://rustup.rs/
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Verify installation
rustc --version
cargo --version
```

#### Git
```bash
# Install from https://git-scm.com/
git --version  # Should show version info
```

### 1.3 Development Tools

#### Recommended IDE
- **VS Code** with extensions:
  - Vue - Official (Volar)
  - Tauri
  - rust-analyzer
  - Prettier

#### Optional Tools
- Git GUI (GitHub Desktop, GitKraken, etc.)
- REST client for API testing

## 2. Project Setup

### 2.1 Clone Repository

```bash
# Clone the repository
git clone https://github.com/mgrabovac22/gitswamp.git
cd gitswamp/software/gitswamp

# Verify directory structure
ls -la  # Should see src, src-tauri, package.json, etc.
```

### 2.2 Install Frontend Dependencies

```bash
# Install npm dependencies
npm install

# Verify installation
npm list

# Expected output shows installed packages:
# gitswamp@0.1.0
# ├── @tauri-apps/api@2.x.x
# ├── vue@3.5.x
# └── (other dependencies)
```

### 2.3 Install Rust Dependencies

```bash
# Rust dependencies are managed by Cargo
# No manual installation needed, but verify:
cargo --version

# Optional: Update Rust to latest
rustup update
```

### 2.4 Project Verification

```bash
# Verify complete setup
npm run build --dry-run  # Check build configuration

# View available scripts
npm run  # Lists available npm scripts
```

## 3. Development Workflow

### 3.1 Start Development Server

```bash
# Terminal 1: Start Vite dev server
npm run dev

# Expected output:
# VITE v6.0.3  ready in 123 ms
# Local:   http://localhost:1420/
# Remote: accessible at your network IP

# Terminal 2: Start Tauri in development mode
npm run tauri dev

# Expected output:
# Building Tauri application...
# Window launched
# Application running at ...
```

### 3.2 Development Cycle

**Workflow for making changes:**

1. **Edit Vue Component**
   ```bash
   # File: src/components/MyComponent.vue
   # Changes auto-reload via Vite HMR
   ```

2. **Edit TypeScript/Composable**
   ```bash
   # File: src/composables/useGit.ts
   # Changes auto-reload and type-checked
   ```

3. **Edit Rust Code**
   ```bash
   # File: src-tauri/src/commands/mycommand.rs
   # Changes require Tauri rebuild
   # Check Terminal 2 for recompilation
   ```

4. **Test Changes**
   - Live reload in Tauri window
   - Check browser console (dev tools)
   - Review Rust compilation errors

### 3.3 Debugging

#### Frontend Debugging

```bash
# Chrome DevTools in Tauri
# Press: Ctrl+Shift+I (Windows/Linux) or Cmd+Option+I (macOS)
# Or: Right-click → Inspect
```

**Available Tools:**
- Elements/Inspector for DOM
- Console for JavaScript
- Network for API calls
- Performance for profiling
- Application for storage

#### Rust Debugging

```bash
# View Rust compiler output in Terminal 2
# Watch for compilation errors and warnings

# Run specific tests
cargo test --lib path::to::test

# View detailed error messages
cargo build --verbose
```

## 4. Building

### 4.1 Development Build

```bash
# Build frontend without optimization
npm run build

# Output:
# dist/
# ├── index.html
# ├── assets/
# │   ├── index-HASH.js
# │   ├── index-HASH.css
# └── (other assets)
```

### 4.2 Production Build

```bash
# Complete build with all optimizations
npm run build && npm run tauri build

# Output:
# src-tauri/target/release/
# ├── gitswamp.exe (Windows)
# ├── gitswamp (Linux)
# └── GitSwamp.app (macOS)
```

### 4.3 Build Options

```bash
# Type check before build
npm run build  # Includes vue-tsc

# Build for specific target
npm run tauri build -- --target x86_64-unknown-linux-gnu

# Create debug build (faster, larger)
npm run tauri build -- --debug
```

## 5. Code Structure for Developers

### 5.1 Adding a New Vue Component

```bash
# Create new component file
touch src/components/MyFeature/NewComponent.vue
```

**Template Structure:**
```vue
<template>
  <div class="new-component">
    <h2>{{ title }}</h2>
    <button @click="handleClick">Click me</button>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";

const title = ref("New Component");

const handleClick = () => {
  console.log("Clicked!");
};
</script>

<style scoped>
.new-component {
  padding: 1rem;
  border: 1px solid #ccc;
}
</style>
```

### 5.2 Adding a New Tauri Command

**1. Create Rust handler (src-tauri/src/commands/myfeature.rs):**
```rust
#[tauri::command]
pub async fn my_new_command(param: String) -> Result<String, String> {
    // Implementation
    Ok(format!("Processed: {}", param))
}
```

**2. Register in lib.rs:**
```rust
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            // ... existing commands
            commands::myfeature::my_new_command,  // Add this
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**3. Call from Vue:**
```typescript
import { invoke } from "@tauri-apps/api/core";

const result = await invoke("my_new_command", {
  param: "test value"
});
```

### 5.3 Adding Type Definitions

**File: src/types/index.ts**
```typescript
export interface MyNewType {
  id: string;
  name: string;
  value: number;
}
```

## 6. Testing

### 6.1 Frontend Testing (When Implemented)

```bash
# Install test framework (if not already)
npm install --save-dev vitest @vue/test-utils

# Run tests
npm run test

# Watch mode
npm run test:watch

# Coverage
npm run test:coverage
```

### 6.2 Rust Testing

```bash
# Run all Rust tests
cargo test

# Run specific test module
cargo test commands::git::

# Run with output
cargo test -- --nocapture

# Run ignored tests
cargo test -- --ignored
```

### 6.3 Integration Testing

```bash
# Build app
npm run build

# Run end-to-end tests (if Cypress/Playwright configured)
npm run test:e2e
```

## 7. Linting & Formatting

### 7.1 TypeScript/Vue Linting

```bash
# Using vue-tsc for type checking
npm run build  # Includes type checking

# With ESLint (if configured)
npm run lint
npm run lint:fix
```

### 7.2 Rust Linting

```bash
# Run clippy for linting
cargo clippy -- -D warnings

# Format code
cargo fmt

# Check formatting without changing
cargo fmt -- --check
```

### 7.3 Overall Code Quality

```bash
# Complete check
cargo fmt --check && cargo clippy && npm run build
```

## 8. Troubleshooting

### 8.1 Common Issues

#### Issue: npm install fails
```bash
# Clear npm cache
npm cache clean --force

# Delete node_modules
rm -rf node_modules package-lock.json

# Reinstall
npm install
```

#### Issue: Rust compilation fails
```bash
# Update Rust
rustup update

# Clean Rust build
cargo clean
cargo build
```

#### Issue: Tauri dev command not found
```bash
# Install Tauri CLI globally
npm install -g @tauri-apps/cli@2

# Or run via npx
npx @tauri-apps/cli@2 dev
```

#### Issue: Port 1420 already in use
```bash
# Find process using port
lsof -i :1420  # macOS/Linux
netstat -ano | findstr :1420  # Windows

# Kill process or change port in vite.config.ts
```

### 8.2 Debug Mode

```bash
# Enable verbose logging
RUST_LOG=debug cargo build
RUST_BACKTRACE=1 cargo run

# Frontend console logs
# Check DevTools console for JavaScript errors
```

### 8.3 Getting Help

- Check existing issues: `https://github.com/mgrabovac22/gitswamp/issues`
- Review documentation in this folder
- Check Tauri documentation: `https://tauri.app/`
- Check Vue 3 documentation: `https://vuejs.org/`

## 9. Environment Variables

### 9.1 Development Environment

**Optional .env file (src-tauri/.env):**
```bash
RUST_LOG=debug
TAURI_DEV_SERVER_URL=http://localhost:1420
```

### 9.2 Git Configuration (for testing)

```bash
# Set git config for commits
git config --global user.name "Your Name"
git config --global user.email "your.email@example.com"
```

## 10. Performance Tips

### 10.1 Development Build Speed

```bash
# Use faster linker (Linux)
# Add to .cargo/config.toml
[build]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

# Incremental compilation (faster rebuilds)
export CARGO_INCREMENTAL=1
```

### 10.2 Development Experience

- Use VS Code with Rust Analyzer for better IDE support
- Enable format on save in VS Code settings
- Use npm run dev for continuous development
- Keep terminal windows visible for error output

## 11. Git Workflow (For Contributors)

```bash
# Create feature branch
git checkout -b feature/my-feature

# Make changes
# ... edit files ...

# Commit with good messages
git add .
git commit -m "feat: describe your feature"

# Push to fork
git push origin feature/my-feature

# Create Pull Request on GitHub
```

---

**Quick Start Command Cheat Sheet:**

```bash
# One-time setup
git clone <repo>
cd gitswamp/software/gitswamp
npm install
rustup update

# Development (in two terminals)
# Terminal 1:
npm run dev

# Terminal 2:
npm run tauri dev

# Building
npm run build
npm run tauri build

# Testing
cargo test
npm run test

# Linting
cargo clippy && npm run build
```

---

**Related Documentation:**
- [20_CODING_STANDARDS.md](./DOCUMENTATION_20_CODING_STANDARDS.md) - Code style guide
- [21_TESTING_GUIDE.md](./DOCUMENTATION_21_TESTING_GUIDE.md) - Testing procedures
- [22_DEPLOYMENT_GUIDE.md](./DOCUMENTATION_22_DEPLOYMENT_GUIDE.md) - Deployment instructions
