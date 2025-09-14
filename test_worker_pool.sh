#!/bin/bash
# Test runner script for worker pool integration tests

set -e  # Exit on any error

echo "🔧 Running Worker Pool Integration Tests"
echo "======================================="

# Set environment variables for testing
export RUST_LOG=info
export SP_DATABASE_URL="sqlite::memory:"

# Function to run test with proper formatting
run_test() {
    local test_name="$1"
    echo
    echo "▶ Running: $test_name"
    echo "----------------------------------------"

    if cargo test --test integration "$test_name" -- --nocapture; then
        echo "✅ $test_name: PASSED"
    else
        echo "❌ $test_name: FAILED"
        return 1
    fi
}

# Function to run performance test
run_perf_test() {
    local test_name="$1"
    echo
    echo "🚀 Running Performance Test: $test_name"
    echo "----------------------------------------"

    # Performance tests get more time and resources
    RUST_LOG=warn cargo test --release --test integration "$test_name" -- --nocapture

    if [ $? -eq 0 ]; then
        echo "✅ $test_name: PASSED"
    else
        echo "❌ $test_name: FAILED"
        return 1
    fi
}

echo "Starting Core Functionality Tests..."
echo

# Core functionality tests
run_test "test_worker_pool_lifecycle"
run_test "test_simple_workflow_execution"
run_test "test_conditional_workflow_execution"
run_test "test_parallel_branch_execution"
run_test "test_delay_scheduling_and_resumption"
run_test "test_crash_recovery"
run_test "test_execution_cancellation"
run_test "test_worker_pool_stats"
run_test "test_concurrent_execution_handling"

echo
echo "Starting Performance Tests..."
echo

# Performance tests (run with release mode for better performance)
run_perf_test "test_high_throughput_simple_workflows"
run_perf_test "test_worker_scaling_performance"
run_perf_test "test_memory_usage_under_load"
run_perf_test "test_error_handling_under_load"
run_perf_test "test_long_running_workflow_performance"
run_perf_test "test_queue_backlog_handling"

echo
echo "Starting Stress Tests..."
echo

# Comprehensive stress test
run_perf_test "test_worker_pool_stress_test"

echo
echo "🎉 All Worker Pool Integration Tests Completed Successfully!"
echo "=========================================================="
echo

# Optional: Run with different configurations
if [ "${RUN_EXTENDED_TESTS:-false}" = "true" ]; then
    echo "Running Extended Configuration Tests..."

    # Test with different worker counts
    for workers in 1 2 4 8 16; do
        echo "Testing with $workers workers..."
        SP_WORKER_COUNT=$workers run_perf_test "test_worker_scaling_performance"
    done
fi

echo "Test Summary:"
echo "- Core functionality: ✅ All tests passed"
echo "- Performance tests: ✅ All tests passed"
echo "- Stress tests: ✅ All tests passed"
echo
echo "Worker pool is ready for production! 🚀"