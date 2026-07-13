# Building Trill Signaling Server for Windows and Linux

This guide covers building for Tier 1 Rust targets: `x86_64-pc-windows-msvc` and `x86_64-unknown-linux-gnu`.

## Prerequisites

### Both Platforms

1. **Rust Toolchain** (1.70+)
   ```bash
   # Windows/Linux installer: https://rustup.rs/
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   
   # Verify installation
   rustc --version
   cargo --version
   ```

2. **Protobuf Compiler** (optional, prost-build handles most cases)
   - Windows: Usually auto-managed by prost-build
   - Linux: `sudo apt-get install protobuf-compiler`

### Windows Specific

1. **Visual Studio Build Tools** or **Visual Studio Community**
   - Download: https://visualstudio.microsoft.com/
   - Select "Desktop Development with C++" workload
   - OR install Visual Studio Build Tools 2022

2. **MSVC Toolchain** (usually default)
   ```bash
   rustup default stable-msvc
   ```

3. **Git for Windows** (recommended)
   - Download: https://git-scm.com/download/win

### Linux Specific

1. **GCC and Build Essentials**
   ```bash
   # Ubuntu/Debian
   sudo apt-get update
   sudo apt-get install -y build-essential pkg-config libssl-dev

   # Fedora/RHEL
   sudo dnf groupinstall "Development Tools"
   sudo dnf install pkg-config openssl-devel

   # Arch
   sudo pacman -S base-devel
   ```

2. **Protobuf Compiler** (optional)
   ```bash
   # Ubuntu/Debian
   sudo apt-get install -y protobuf-compiler

   # Fedora/RHEL
   sudo dnf install protobuf-compiler

   # Arch
   sudo pacman -S protobuf
   ```

## Building on Windows

### Option 1: Native Build (Recommended)

```bash
# Clone and navigate to rust directory
cd rust

# Build for Windows (x86_64-pc-windows-msvc)
cargo build --release

# Binary location
# target\release\trill-signaling-server.exe
```

### Option 2: Cross-Compile from Windows to Linux

```bash
# Add Linux target
rustup target add x86_64-unknown-linux-gnu

# Install cross-compilation tools (optional, for optimization)
# Or use cargo with custom linker

# Build for Linux
cargo build --release --target x86_64-unknown-linux-gnu

# Binary location
# target\x86_64-unknown-linux-gnu\release\trill-signaling-server
```

**Note**: Cross-compiling from Windows to Linux requires additional setup with `cross` tool or custom linker configuration. Native Linux build is recommended for production.

## Building on Linux

### Option 1: Native Build (Recommended)

```bash
# Clone and navigate to rust directory
cd rust

# Build for Linux (x86_64-unknown-linux-gnu)
cargo build --release

# Binary location
# target/release/trill-signaling-server
```

### Option 2: Cross-Compile from Linux to Windows

This is NOT recommended without significant setup. Use Windows to build for Windows.

```bash
# Add Windows target (requires mingw-w64)
rustup target add x86_64-pc-windows-gnu

# Install MinGW (Ubuntu/Debian example)
sudo apt-get install -y mingw-w64

# Build for Windows
cargo build --release --target x86_64-pc-windows-gnu

# Binary location
# target/x86_64-pc-windows-gnu/release/trill-signaling-server.exe
```

## Quick Start Scripts

### build.cmd (Windows)

```batch
@echo off
setlocal enabledelayedexpansion

echo Building Trill Signaling Server for Windows...
rustup default stable-msvc
cargo build --release

if %ERRORLEVEL% EQU 0 (
    echo.
    echo Build successful!
    echo Binary: target\release\trill-signaling-server.exe
) else (
    echo Build failed!
    exit /b 1
)
```

### build.sh (Linux)

```bash
#!/bin/bash
set -e

echo "Building Trill Signaling Server for Linux..."

cargo build --release

echo ""
echo "Build successful!"
echo "Binary: target/release/trill-signaling-server"
```

Usage:
```bash
chmod +x build.sh
./build.sh
```

## Docker Multi-Platform Build

Build for both Windows and Linux using Docker buildx:

```bash
# Build and push to registry
docker buildx build \
  --platform linux/amd64 \
  -t myregistry/trill-signaling:latest \
  -f Dockerfile .

# Build locally
docker build -t trill-signaling:latest .
```

## Verification

### Windows

```cmd
# Run the server
target\release\trill-signaling-server.exe

# In another terminal, test
curl http://localhost:8000/health
```

### Linux

```bash
# Run the server
./target/release/trill-signaling-server

# In another terminal, test
curl http://localhost:8000/health
```

Both should respond with:
```json
{"status":"ok"}
```

## Troubleshooting

### Build Issues

**Error: "error: linker `link.exe` not found"** (Windows)
- Install Visual Studio Build Tools with C++ support
- Or install MSVC via: `rustup toolchain install stable-msvc`

**Error: "error: Microsoft Visual C++ 14.0 is required"** (Windows)
- Download and install Visual Studio Build Tools 2022
- https://visualstudio.microsoft.com/downloads/

**Error: "error: linker 'cc' not found"** (Linux)
- Ubuntu/Debian: `sudo apt-get install build-essential`
- Fedora: `sudo dnf groupinstall "Development Tools"`

### Runtime Issues

**Port Already in Use**
```bash
# Change port via environment variable
export SERVER_PORT=8001  # Linux
set SERVER_PORT=8001     # Windows

# Or edit .env file
```

**Protobuf Compilation Fails**
- Clear build cache: `cargo clean`
- Ensure `proto/signaling.proto` exists
- Rebuild: `cargo build --release`

## Release Build Optimization

The release profile is configured for production:

```toml
[profile.release]
opt-level = 3        # Maximum optimization
lto = true           # Link Time Optimization
codegen-units = 1    # Single code generation unit
```

This may take longer to compile but produces highly optimized binaries.

For faster development builds, use:
```bash
cargo build          # Debug build (much faster)
cargo run            # Run in debug mode
```

## System Requirements for Binary Execution

### Windows

- Windows 7 SP1 or later (x86_64)
- MSVC Runtime (usually pre-installed)
- No additional dependencies

### Linux

- glibc 2.17+ (any modern Linux)
- OpenSSL development libraries (for HTTPS requests)

If binary fails on Linux:
```bash
ldd target/release/trill-signaling-server
# Check for missing libraries and install if needed
```

## Platform Support Matrix

| Platform | Target Triple | Status | Build Time | Binary Size |
|----------|---|---|---|---|
| Windows 10/11 (x86_64) | `x86_64-pc-windows-msvc` | ✅ Tier 1 | ~2-3 min | ~50-80 MB |
| Linux (x86_64) | `x86_64-unknown-linux-gnu` | ✅ Tier 1 | ~2-3 min | ~30-50 MB |
| macOS (x86_64) | `x86_64-apple-darwin` | ✅ Tier 1 | ~2-3 min | ~30-50 MB |
| macOS (ARM64) | `aarch64-apple-darwin` | ✅ Tier 1 | ~3-5 min | ~30-50 MB |

To add support for other platforms:
```bash
rustup target list --installed
rustup target add <target-triple>
cargo build --release --target <target-triple>
```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Build

on: [push, pull_request]

jobs:
  build-windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true
      - run: cargo build --release --manifest-path rust/Cargo.toml
      - uses: actions/upload-artifact@v3
        with:
          name: trill-signaling-server-windows
          path: rust/target/release/trill-signaling-server.exe

  build-linux:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true
      - run: cargo build --release --manifest-path rust/Cargo.toml
      - uses: actions/upload-artifact@v3
        with:
          name: trill-signaling-server-linux
          path: rust/target/release/trill-signaling-server
```

## See Also

- [Rust Platform Support](https://doc.rust-lang.org/nightly/rustc/platform-support.html)
- [Cross Compilation Guide](https://rust-lang.github.io/rustup/cross-compilation.html)
- [Cargo Build Documentation](https://doc.rust-lang.org/cargo/commands/cargo-build.html)
