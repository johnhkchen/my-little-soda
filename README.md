# My Little Soda

My Little Soda is an autonomous coding agent system designed for multi-agent orchestration in GitHub issue processing. It provides a comprehensive workflow for automated issue resolution with integrated release management.

## Features

- **Autonomous Agent System**: Multi-agent coordination for GitHub issue processing
- **Cross-Platform Binary Releases**: Automated builds for Linux, macOS, Windows, and Raspberry Pi
- **Enhanced Cross-Compilation**: Uses cross-rs for robust ARM target support
- **Universal macOS Binaries**: Single binary supporting both Intel and Apple Silicon
- **Automated Release Pipeline**: Version tag-triggered releases with comprehensive asset generation

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