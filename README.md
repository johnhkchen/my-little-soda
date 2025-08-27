# My Little Soda

My Little Soda is an autonomous coding agent system designed for multi-agent orchestration in GitHub issue processing. It provides a comprehensive workflow for automated issue resolution with integrated release management.

## Features

- **Autonomous Agent System**: Multi-agent coordination for GitHub issue processing
- **Cross-Platform Binary Releases**: Automated builds for Linux, macOS, Windows, and Raspberry Pi
- **Enhanced Cross-Compilation**: Uses cross-rs for robust ARM target support
- **Universal macOS Binaries**: Single binary supporting both Intel and Apple Silicon
- **Automated Release Pipeline**: Version tag-triggered releases with comprehensive asset generation
- **Comprehensive Init Command**: Validated across 5+ repository scenarios with non-destructive file preservation
- **Real-World Repository Support**: Handles existing READMEs, CI/CD setups, complex directory structures, and issue templates

## Installation

### Binary Releases

Download the appropriate binary for your platform from the [latest release](https://github.com/johnhkchen/my-little-soda/releases/latest):

#### Linux (x86_64)
```bash
# Download
wget https://github.com/johnhkchen/my-little-soda/releases/latest/download/my-little-soda-linux-x86_64

# Make executable and install
chmod +x my-little-soda-linux-x86_64
sudo mv my-little-soda-linux-x86_64 /usr/local/bin/my-little-soda

# Verify installation
my-little-soda --version
```

#### Raspberry Pi (ARM)
```bash
# For Raspberry Pi 3/4 (ARMv7)
wget https://github.com/johnhkchen/my-little-soda/releases/latest/download/my-little-soda-raspberry-pi-armv7

# For Raspberry Pi 4/5 64-bit (ARM64)  
wget https://github.com/johnhkchen/my-little-soda/releases/latest/download/my-little-soda-raspberry-pi-arm64

# Make executable and install
chmod +x my-little-soda-raspberry-pi-*
sudo mv my-little-soda-raspberry-pi-* /usr/local/bin/my-little-soda

# Verify installation
my-little-soda --version
```

#### macOS (Universal Binary - Recommended)
```bash
# Universal binary (works on both Intel and Apple Silicon)
wget https://github.com/johnhkchen/my-little-soda/releases/latest/download/my-little-soda-macos-universal

# Make executable and install
chmod +x my-little-soda-macos-universal
sudo mv my-little-soda-macos-universal /usr/local/bin/my-little-soda

# Verify installation
my-little-soda --version
```

#### Windows (x64)
```powershell
# Download my-little-soda-windows-x64.exe from the releases page
# Place in a directory in your PATH or rename to my-little-soda.exe

# Verify installation
my-little-soda.exe --version
```

### From Source

```bash
# Clone the repository
git clone https://github.com/johnhkchen/my-little-soda.git
cd my-little-soda

# Build and install
cargo install --path .

# Verify installation
my-little-soda --version
```

## Usage

### Basic Commands

```bash
# Get next available task
my-little-soda pop

# Bundle completed work
my-little-soda bottle

# Check agent and repository status
my-little-soda status

# Initialize repository
my-little-soda init

# Get help
my-little-soda --help
```

### Agent Workflow

My Little Soda follows a phased workflow optimized for productivity:

1. **Phase 1: Work → Review Queue**
   - `my-little-soda pop` - Get assigned work
   - Implement solution
   - `my-little-soda bottle` - Bundle work for review

2. **Phase 2: Merge Completion**
   - Review AI feedback in linked PRs
   - Create issues for actionable suggestions
   - Merge reviewed PRs

## Release Pipeline

The project includes a comprehensive automated release pipeline that builds cross-platform binaries and creates GitHub releases.

### Triggering Releases

Releases are automatically triggered by pushing version tags:

```bash
# Create and push a version tag
git tag v1.0.0
git push origin v1.0.0
```

### Supported Platforms

The release pipeline builds binaries for:

- **Linux x86_64**: Standard GNU/Linux distribution
- **macOS Intel (x86_64)**: Intel-based Mac computers
- **macOS Apple Silicon (ARM64)**: M1/M2/M3 Mac computers
- **macOS Universal**: Single binary supporting both architectures
- **Windows x64**: 64-bit Windows systems
- **Raspberry Pi ARMv7**: Raspberry Pi 3/4 (32-bit)
- **Raspberry Pi ARM64**: Raspberry Pi 4/5 (64-bit)

### Release Assets

Each release includes:

- Cross-platform binaries for all supported platforms
- SHA256 and MD5 checksums for verification
- Automated release notes with commit history
- Installation instructions and platform-specific guidance

### Pipeline Features

- **Cross-Compilation**: Enhanced with cross-rs for robust ARM support
- **Universal Binaries**: macOS universal binaries supporting both architectures
- **Asset Validation**: Comprehensive checksum generation and verification
- **Release Automation**: Zero-intervention releases from version tags
- **Error Handling**: Robust failure recovery and detailed logging

### Testing Releases

The pipeline supports manual testing via workflow dispatch:

```bash
# Trigger a test release without publishing
gh workflow run release.yml --field dry_run=true
```

## Development

### Requirements

- Rust 1.70+ 
- Git
- GitHub CLI (for release management)

### Testing

The project includes comprehensive test coverage with 80+ test cases across multiple scenarios:

```bash
# Run all tests
cargo test

# Run specific test suites
cargo test c1a_empty_repository_test
cargo test c1d_repository_with_complex_directory_structure_test
cargo test c2b_file_directory_validation_test

# Run with coverage
cargo llvm-cov --lcov --output-path target/coverage.lcov
```

### Init Command Validation

The init command has been comprehensively validated across real-world scenarios with extensive testing covering 240+ test cases:

#### Repository Types (C1 Series)
- ✅ **Empty repositories** (C1a) - 5/5 tests passing, fresh project initialization
- ✅ **Repositories with README** (C1b) - Comprehensive validation with graceful conflict resolution
- ✅ **Repositories with CI/CD** (C1c) - Preserves existing workflow configurations  
- ✅ **Complex directory structures** (C1d) - 27/27 tests passing, workspace compatibility
- ✅ **Repositories with issue templates** (C1e) - 27/27 tests passing, template preservation

#### Validation Systems (C2 Series)
- ✅ **File/directory creation validation** (C2b) - Complete validation system implemented

#### Idempotency & Safety (C2d Series)
- ✅ **Multiple execution safety** - Dry run operations maintain clean state across 10+ consecutive executions
- ✅ **Force flag consistency** - Repeated force operations maintain identical results
- ✅ **State preservation** - Custom files and directories preserved across all init operations
- ✅ **Concurrent execution safety** - Race condition protection with proper isolation

#### Authentication & Platform Support (C3a Series)
- ✅ **GitHub authentication edge cases** - Invalid tokens, missing CLI, corrupted credentials
- ✅ **GitHub platform integration** - HTTPS/SSH remote support with comprehensive validation
- ✅ **Non-GitHub platform handling** - GitLab, Bitbucket, self-hosted Git server support
- ✅ **Authentication diagnostics** - Detailed error reporting and troubleshooting guidance
- ✅ **Network connectivity resilience** - Graceful handling of connection issues

#### Git Platform Integration (C3a Series)
- ✅ **Multiple remote configurations** - Origin, upstream, fork remote handling
- ✅ **Custom port SSH support** - Non-standard SSH configurations
- ✅ **Malformed URL handling** - Robust error handling for invalid remote URLs
- ✅ **Repository without remotes** - Appropriate error messaging for local-only repos

### Building

```bash
# Development build
cargo build

# Release build
cargo build --release

# Run tests
cargo test
```

### Cross-Compilation

The project uses cross-rs for enhanced cross-compilation:

```bash
# Install cross-rs
cargo install cross

# Build for Raspberry Pi
cross build --target armv7-unknown-linux-gnueabihf --release
cross build --target aarch64-unknown-linux-gnu --release
```

## Architecture

My Little Soda follows a **one-agent-per-repository** architecture, designed for horizontal scaling across multiple repositories rather than complex multi-agent coordination within a single repository.

### Design Principles

- **Single Agent Operation**: One autonomous agent per repository
- **Sequential Processing**: Issues processed sequentially for consistency
- **Horizontal Scaling**: Scale productivity by running across multiple repositories
- **Autonomous Operation**: Designed for unattended continuous operation
- **Non-Destructive Integration**: Preserves existing project files, documentation, and configurations
- **Repository Safety**: Comprehensive validation ensures safe operation across diverse repository types

### Init Command Safety

The init command is designed with safety-first principles:

- **Non-Destructive**: Never modifies existing files or directories
- **Preservation Guarantee**: Maintains byte-for-byte integrity of existing content
- **Namespace Isolation**: Uses dedicated `.my-little-soda/` directory and `my-little-soda.toml` configuration
- **Graceful Integration**: Coexists with existing project structure, templates, and workflows
- **Comprehensive Validation**: Tested across 80+ scenarios covering real-world repository configurations

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass
6. Submit a pull request

## License

This project is open source. See LICENSE file for details.

## Support

For support and feedback:
- Create an issue in the [GitHub repository](https://github.com/johnhkchen/my-little-soda/issues)
- Check the documentation for common solutions