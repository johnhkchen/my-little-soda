#!/usr/bin/env bash
# Coverage reporting script for My Little Soda
# Generates comprehensive coverage reports in multiple formats

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
COVERAGE_DIR="target/coverage"
HTML_DIR="$COVERAGE_DIR/html"
LCOV_FILE="$COVERAGE_DIR/coverage.lcov"
JSON_FILE="$COVERAGE_DIR/coverage.json"
COBERTURA_FILE="$COVERAGE_DIR/coverage.xml"

# Coverage thresholds
MIN_LINE_COVERAGE=70
MIN_FUNCTION_COVERAGE=75
MIN_REGION_COVERAGE=65

# Parse command line arguments
SCOPE="lib"
FORMAT="html"
OPEN_REPORT=false
FAIL_ON_THRESHOLD=false
VERBOSE=false

usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Generate code coverage reports for My Little Soda

OPTIONS:
    --scope SCOPE       Coverage scope: lib, all, integration (default: lib)
    --format FORMAT     Output format: html, lcov, json, cobertura, all (default: html)
    --open             Open HTML report in browser after generation
    --fail-on-threshold Exit with error if coverage thresholds not met
    --verbose          Enable verbose output
    --help             Show this help message

EXAMPLES:
    $0                                   # Generate HTML report for lib tests
    $0 --scope all --format all          # Generate all formats for all tests
    $0 --format lcov --fail-on-threshold # Generate LCOV with threshold checking
    $0 --open                           # Generate HTML report and open in browser

EOF
}

while [[ $# -gt 0 ]]; do
    case $1 in
        --scope)
            SCOPE="$2"
            shift 2
            ;;
        --format)
            FORMAT="$2"
            shift 2
            ;;
        --open)
            OPEN_REPORT=true
            shift
            ;;
        --fail-on-threshold)
            FAIL_ON_THRESHOLD=true
            shift
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --help)
            usage
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}" >&2
            usage
            exit 1
            ;;
    esac
done

# Validate inputs
case $SCOPE in
    lib|all|integration) ;;
    *)
        echo -e "${RED}Invalid scope: $SCOPE. Must be one of: lib, all, integration${NC}" >&2
        exit 1
        ;;
esac

case $FORMAT in
    html|lcov|json|cobertura|all) ;;
    *)
        echo -e "${RED}Invalid format: $FORMAT. Must be one of: html, lcov, json, cobertura, all${NC}" >&2
        exit 1
        ;;
esac

# Function to log messages
log() {
    if [[ "$VERBOSE" == true ]]; then
        echo -e "$1"
    fi
}

print_step() {
    echo -e "${BLUE}📊 $1${NC}"
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

# Check if cargo-llvm-cov is installed
check_dependencies() {
    print_step "Checking dependencies..."
    
    if ! command -v cargo-llvm-cov &> /dev/null; then
        print_error "cargo-llvm-cov is not installed. Installing..."
        cargo install cargo-llvm-cov
    fi
    
    if ! rustup component list --installed | grep -q llvm-tools; then
        print_error "llvm-tools-preview not installed. Installing..."
        rustup component add llvm-tools-preview
    fi
    
    # Set LLVM tool paths if not already set
    if [[ -z "${LLVM_COV:-}" ]] || [[ -z "${LLVM_PROFDATA:-}" ]]; then
        log "Setting LLVM tool paths..."
        
        # Get current toolchain
        TOOLCHAIN=$(rustup show active-toolchain | cut -d' ' -f1)
        RUST_HOME="$HOME/.rustup/toolchains/$TOOLCHAIN"
        
        if [[ -f "$RUST_HOME/lib/rustlib/x86_64-unknown-linux-gnu/bin/llvm-cov" ]]; then
            export LLVM_COV="$RUST_HOME/lib/rustlib/x86_64-unknown-linux-gnu/bin/llvm-cov"
            export LLVM_PROFDATA="$RUST_HOME/lib/rustlib/x86_64-unknown-linux-gnu/bin/llvm-profdata"
            log "LLVM tools found: $LLVM_COV"
        else
            print_warning "Could not find LLVM tools. They may need to be set manually."
        fi
    fi
    
    print_success "Dependencies verified"
}

# Prepare coverage directory
setup_directories() {
    print_step "Setting up coverage directories..."
    rm -rf "$COVERAGE_DIR"
    mkdir -p "$COVERAGE_DIR"
    log "Created coverage directory: $COVERAGE_DIR"
}

# Build scope arguments
build_scope_args() {
    case $SCOPE in
        lib)
            echo "--lib"
            ;;
        all)
            echo "--all-targets"
            ;;
        integration)
            echo "--tests"
            ;;
    esac
}

# Generate coverage report in specified format
generate_coverage() {
    local format=$1
    local scope_args
    scope_args=$(build_scope_args)
    
    print_step "Generating $format coverage report (scope: $SCOPE)..."
    
    case $format in
        html)
            cargo llvm-cov $scope_args --html --output-dir "$HTML_DIR"
            print_success "HTML report generated: $HTML_DIR/index.html"
            ;;
        lcov)
            cargo llvm-cov $scope_args --lcov --output-path "$LCOV_FILE"
            print_success "LCOV report generated: $LCOV_FILE"
            ;;
        json)
            cargo llvm-cov $scope_args --json --output-path "$JSON_FILE"
            print_success "JSON report generated: $JSON_FILE"
            ;;
        cobertura)
            cargo llvm-cov $scope_args --cobertura --output-path "$COBERTURA_FILE"
            print_success "Cobertura XML report generated: $COBERTURA_FILE"
            ;;
    esac
}

# Check coverage thresholds
check_thresholds() {
    if [[ "$FAIL_ON_THRESHOLD" == false ]]; then
        return 0
    fi
    
    print_step "Checking coverage thresholds..."
    
    local scope_args
    scope_args=$(build_scope_args)
    local threshold_failed=false
    
    # Check line coverage
    if ! cargo llvm-cov $scope_args --fail-under-lines "$MIN_LINE_COVERAGE" --summary-only &>/dev/null; then
        print_error "Line coverage below threshold: $MIN_LINE_COVERAGE%"
        threshold_failed=true
    fi
    
    # Check function coverage
    if ! cargo llvm-cov $scope_args --fail-under-functions "$MIN_FUNCTION_COVERAGE" --summary-only &>/dev/null; then
        print_error "Function coverage below threshold: $MIN_FUNCTION_COVERAGE%"
        threshold_failed=true
    fi
    
    # Check region coverage
    if ! cargo llvm-cov $scope_args --fail-under-regions "$MIN_REGION_COVERAGE" --summary-only &>/dev/null; then
        print_error "Region coverage below threshold: $MIN_REGION_COVERAGE%"
        threshold_failed=true
    fi
    
    if [[ "$threshold_failed" == true ]]; then
        print_error "Coverage thresholds not met"
        return 1
    fi
    
    print_success "All coverage thresholds met"
}

# Display coverage summary
show_summary() {
    print_step "Coverage Summary:"
    local scope_args
    scope_args=$(build_scope_args)
    cargo llvm-cov $scope_args --summary-only
}

# Open HTML report in browser
open_html_report() {
    if [[ "$OPEN_REPORT" == true ]] && [[ -f "$HTML_DIR/index.html" ]]; then
        print_step "Opening coverage report in browser..."
        
        if command -v xdg-open &> /dev/null; then
            xdg-open "$HTML_DIR/index.html"
        elif command -v open &> /dev/null; then
            open "$HTML_DIR/index.html"
        else
            print_warning "Could not open browser. View report at: file://$PWD/$HTML_DIR/index.html"
        fi
    fi
}

# Main execution
main() {
    echo -e "${BLUE}🚀 My Little Soda Coverage Reporter${NC}"
    echo "Scope: $SCOPE | Format: $FORMAT | Thresholds: $([ "$FAIL_ON_THRESHOLD" == true ] && echo "enforced" || echo "informational")"
    echo
    
    check_dependencies
    setup_directories
    
    # Generate reports based on format
    if [[ "$FORMAT" == "all" ]]; then
        generate_coverage "html"
        generate_coverage "lcov"
        generate_coverage "json"
        generate_coverage "cobertura"
    else
        generate_coverage "$FORMAT"
    fi
    
    show_summary
    check_thresholds
    open_html_report
    
    echo
    print_success "Coverage reporting complete!"
    
    if [[ "$FORMAT" == "html" ]] || [[ "$FORMAT" == "all" ]]; then
        echo -e "View HTML report: ${BLUE}file://$PWD/$HTML_DIR/index.html${NC}"
    fi
    
    if [[ -f "$LCOV_FILE" ]]; then
        echo -e "LCOV report: ${BLUE}$LCOV_FILE${NC}"
    fi
}

# Run main function
main "$@"