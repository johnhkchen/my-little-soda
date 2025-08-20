# Clambake Docker Infrastructure

> **Containerized observability. Isolated environments. Zero state drift.**

## Docker Components

Clambake uses Docker for observability infrastructure and isolated testing environments. No application state is ever stored in containers.

## Available Images

### Main Application (`Dockerfile`)
```dockerfile
# Multi-stage build for minimal image
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/clambake /usr/local/bin/
# No state files, no config files
# GitHub is the only source of truth
ENTRYPOINT ["clambake"]
```

### Phoenix Observability (`Dockerfile.phoenix`)
```dockerfile
FROM arizephoenix/phoenix:latest
# Pre-configured for multi-agent tracing
# No custom configuration needed
# All metrics flow through OTLP
```

## Docker Compose Stacks

### Development Stack (`docker-compose.yml`)
```yaml
version: '3.8'

services:
  phoenix:
    build:
      context: .
      dockerfile: Dockerfile.phoenix
    ports:
      - "6006:6006"  # Phoenix UI
      - "4317:4317"  # OTLP gRPC
      - "4318:4318"  # OTLP HTTP
    volumes:
      - phoenix-data:/data
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:6006/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  # No database service - GitHub is the database
  # No cache service - GitHub is the source of truth
  # No state storage - Everything is derived

volumes:
  phoenix-data:
    # Observability data only, no application state
```

### Test Infrastructure (`docker-compose.test.yml`)
```yaml
version: '3.8'

services:
  test-runner:
    build:
      context: .
      dockerfile: Dockerfile
      target: test
    environment:
      # No environment variables for configuration
      - RUST_LOG=debug
      - RUST_BACKTRACE=1
    command: cargo test --all-features

  github-mock:
    image: clambake/github-mock:latest
    ports:
      - "8080:8080"
    # Simulates GitHub API for testing

  chaos-injector:
    image: clambake/chaos:latest
    # Injects failures for resilience testing
```

## Running with Docker

### Development
```bash
# Start Phoenix observability
docker-compose up -d phoenix

# Run Clambake with Phoenix integration
docker run --rm \
  --network clambake_default \
  -v $PWD:/workspace \
  clambake:latest route --trace

# View traces in Phoenix
open http://localhost:6006
```

### Testing
```bash
# Run all tests in containers
docker-compose -f docker-compose.test.yml up --abort-on-container-exit

# Run chaos tests
docker-compose -f docker-compose.test.yml run chaos-injector

# Clean up test containers
docker-compose -f docker-compose.test.yml down -v
```

### Production
```bash
# Deploy Phoenix for production monitoring
docker stack deploy -c docker-compose.yml clambake

# No application containers in production
# Clambake runs as a CLI tool, not a service
```

## Container Principles

### VERBOTEN in Containers
- ❌ No application state storage
- ❌ No configuration files
- ❌ No environment variable configs
- ❌ No database containers
- ❌ No manual synchronization

### Required Patterns
- ✅ Observability infrastructure only
- ✅ Ephemeral test environments
- ✅ Isolated agent workspaces
- ✅ Health checks for all services
- ✅ Automatic cleanup

## Building Images

```bash
# Build main application
docker build -t clambake:latest .

# Build Phoenix stack
docker build -f Dockerfile.phoenix -t clambake-phoenix:latest .

# Build test infrastructure
docker build -f Dockerfile.test -t clambake-test:latest .

# Multi-platform build
docker buildx build --platform linux/amd64,linux/arm64 -t clambake:latest .
```

## Image Security

```bash
# Scan for vulnerabilities
docker scan clambake:latest

# Minimal base images only
# No unnecessary packages
# Non-root user execution
# Read-only filesystems where possible
```

## Docker Best Practices

### Image Size
- Multi-stage builds
- Minimal base images
- No build tools in final image
- Compressed layers

### Security
- Regular base image updates
- Vulnerability scanning
- No secrets in images
- Least privilege principle

### Observability
- All containers emit OTLP
- Structured logging
- Health endpoints
- Resource metrics

## Troubleshooting

```bash
# Check container logs
docker-compose logs -f phoenix

# Inspect running containers
docker ps -a

# Clean up unused resources
docker system prune -a

# Reset Phoenix data
docker volume rm clambake_phoenix-data
```

---

**Containers for infrastructure, not state. GitHub remains the truth.**