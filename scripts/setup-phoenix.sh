#!/bin/bash
# Setup script for Phoenix observability dashboard

set -e

echo "🔭 Setting up Phoenix Observability Dashboard..."

# Check if Docker is available
if ! command -v docker &> /dev/null; then
    echo "❌ Docker is required but not installed. Please install Docker first."
    exit 1
fi

echo "📦 Starting Phoenix container..."

# Stop existing container if running
docker stop phoenix-clambake 2>/dev/null || true
docker rm phoenix-clambake 2>/dev/null || true

# Create Phoenix data volume
docker volume create phoenix-data || true

# Start Phoenix container
docker run -d \
    --name phoenix-clambake \
    --restart unless-stopped \
    -p 6006:6006 \
    -p 4317:4317 \
    -p 4318:4318 \
    -v phoenix-data:/phoenix-data \
    -e PHOENIX_WORKING_DIR=/phoenix-data \
    -e PHOENIX_HOST=0.0.0.0 \
    -e PHOENIX_PORT=6006 \
    arizephoenix/phoenix:latest

echo "⏳ Waiting for Phoenix to start..."
sleep 10

# Check if Phoenix is running
if curl -s http://localhost:6006/health > /dev/null; then
    echo "✅ Phoenix is running successfully!"
    echo "📊 Dashboard: http://localhost:6006"
    echo "🔌 OTLP gRPC endpoint: http://localhost:4317"
    echo "🔌 OTLP HTTP endpoint: http://localhost:4318"
else
    echo "❌ Phoenix failed to start. Check Docker logs:"
    docker logs phoenix-clambake
    exit 1
fi

echo ""
echo "🚀 Phoenix setup complete!"
echo "💡 Run 'cargo run' to start Clambake with telemetry enabled"
echo "📈 View traces and metrics at: http://localhost:6006"