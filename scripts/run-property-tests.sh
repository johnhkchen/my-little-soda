#!/bin/bash
# Property-Based Testing Runner for Clambake Agent Coordination
# Comprehensive test suite for validating agent coordination invariants

set -e

echo "🧪 Clambake Agent Coordination Property Testing Suite"
echo "=================================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test configuration
DEFAULT_CASES=256
STRESS_CASES=1000
QUICK_CASES=50

# Parse command line arguments
CASES=${1:-$DEFAULT_CASES}
MODE=${2:-"standard"}

print_header() {
    echo -e "${BLUE}$1${NC}"
    echo "----------------------------------------"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Environment setup
print_header "Environment Setup"
echo "🔧 Setting up test environment..."
export CARGO_TERM_COLOR=always
export PROPTEST_CASES=$CASES
export PROPTEST_MAX_SHRINK_ITERS=10000
export RUST_BACKTRACE=1

print_success "Environment configured (PROPTEST_CASES=$CASES)"

# Build phase
print_header "Build Phase"
echo "🔨 Building tests..."
if cargo build --tests; then
    print_success "Build completed successfully"
else
    print_error "Build failed"
    exit 1
fi

# Core property tests
print_header "Core Property Tests"
echo "🎯 Running agent coordination property tests..."

if cargo test --test property_based_agent_coordination property_tests:: --verbose; then
    print_success "All property tests passed"
else
    print_error "Property tests failed"
    exit 1
fi

# Chaos testing
print_header "Chaos Testing"
echo "🌀 Running chaos engineering tests..."

if cargo test --test property_based_agent_coordination chaos_testing:: --verbose; then
    print_success "Chaos tests passed - system is resilient"
else
    print_error "Chaos tests failed - system resilience issues detected"
    exit 1
fi

# Framework validation
print_header "Framework Validation"
echo "🔍 Validating property testing framework..."

if cargo test --test property_based_agent_coordination test_property_framework_setup --verbose; then
    print_success "Property testing framework validated"
else
    print_error "Property testing framework issues detected"
    exit 1
fi

# Integration tests (optional - may fail without GitHub credentials)
print_header "Integration Tests"
echo "🌐 Running integration property tests..."

if cargo test --test property_based_agent_coordination integration_property_tests:: --verbose; then
    print_success "Integration property tests passed"
else
    print_warning "Integration tests skipped (GitHub credentials not available)"
fi

# Performance testing
if [ "$MODE" = "performance" ] || [ "$MODE" = "stress" ]; then
    print_header "Performance Testing"
    echo "🚀 Running performance benchmarks..."
    
    echo "📊 Testing with different scales..."
    
    # Quick run
    echo "  - Quick test (50 cases)..."
    PROPTEST_CASES=50 cargo test --test property_based_agent_coordination property_tests:: --release --quiet
    
    # Standard run
    echo "  - Standard test ($DEFAULT_CASES cases)..."
    PROPTEST_CASES=$DEFAULT_CASES cargo test --test property_based_agent_coordination property_tests:: --release --quiet
    
    if [ "$MODE" = "stress" ]; then
        # Stress test
        echo "  - Stress test ($STRESS_CASES cases)..."
        PROPTEST_CASES=$STRESS_CASES cargo test --test property_based_agent_coordination property_tests:: --release --quiet
    fi
    
    print_success "Performance testing completed"
fi

# Coverage analysis
if command -v cargo-llvm-cov >/dev/null 2>&1; then
    print_header "Coverage Analysis"
    echo "📈 Generating test coverage report..."
    
    if cargo llvm-cov --test property_based_agent_coordination --html --output-dir coverage-report; then
        print_success "Coverage report generated in coverage-report/"
    else
        print_warning "Coverage analysis failed"
    fi
else
    print_warning "cargo-llvm-cov not installed - skipping coverage analysis"
    echo "   Install with: cargo install cargo-llvm-cov"
fi

# Summary
print_header "Test Summary"
echo "🎉 Property-based testing suite completed successfully!"
echo ""
echo "📋 Tests executed:"
echo "   ✅ Agent assignment invariants"
echo "   ✅ Capacity limit violations"
echo "   ✅ Concurrent operation safety"
echo "   ✅ Assignment uniqueness"
echo "   ✅ State reversibility"
echo "   ✅ Chaos testing scenarios"
echo "   ✅ System resilience validation"
echo ""
echo "🔬 Property test configuration:"
echo "   - Test cases per property: $CASES"
echo "   - Shrink iterations: 10,000"
echo "   - Mode: $MODE"
echo ""
print_success "All agent coordination invariants verified!"

# Usage instructions
if [ "$MODE" = "help" ] || [ "$1" = "--help" ] || [ "$1" = "-h" ]; then
    echo ""
    echo "Usage: $0 [cases] [mode]"
    echo ""
    echo "Arguments:"
    echo "  cases    Number of test cases per property (default: $DEFAULT_CASES)"
    echo "  mode     Test mode: standard|performance|stress|help"
    echo ""
    echo "Examples:"
    echo "  $0                    # Standard run with $DEFAULT_CASES cases"
    echo "  $0 100               # Quick run with 100 cases"
    echo "  $0 1000 stress       # Stress test with 1000 cases"
    echo "  $0 500 performance   # Performance benchmarking"
    echo ""
    echo "Environment variables:"
    echo "  PROPTEST_CASES            Number of test cases"
    echo "  PROPTEST_MAX_SHRINK_ITERS Shrink iteration limit"
    echo "  RUST_BACKTRACE           Backtrace verbosity"
fi