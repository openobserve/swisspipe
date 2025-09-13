#!/bin/bash

# Performance testing script for SwissPipe workflow API using hyperfine
# Tests the workflow trigger endpoint with various configurations

set -e

# Configuration
WORKFLOW_ID="03b91fbf-535c-43b9-b553-31534b15cf1a"
BASE_URL="http://localhost:3700"
ENDPOINT="/api/v1/${WORKFLOW_ID}/trigger"
FULL_URL="${BASE_URL}${ENDPOINT}"

# Test data payload
TEST_PAYLOAD='{
  "company_name": "Test Performance Company",
  "linkedin": "https://www.linkedin.com/company/openobserve/",
  "website": "https://example.com",
  "user_name": "Performance Test User",
  "employees": 10
}'

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}SwissPipe Performance Test Suite${NC}"
echo -e "${BLUE}=================================${NC}"
echo ""
echo -e "Testing endpoint: ${GREEN}${FULL_URL}${NC}"
echo -e "Workflow ID: ${GREEN}${WORKFLOW_ID}${NC}"
echo ""

# Check if hyperfine is installed
if ! command -v hyperfine &> /dev/null; then
    echo -e "${RED}Error: hyperfine is not installed${NC}"
    echo -e "${YELLOW}Install with: brew install hyperfine${NC}"
    exit 1
fi

# Check if server is running
echo -e "${YELLOW}Checking if server is running...${NC}"
if ! curl -s --connect-timeout 5 "${BASE_URL}/health" > /dev/null 2>&1; then
    echo -e "${RED}Error: Server is not running at ${BASE_URL}${NC}"
    echo -e "${YELLOW}Start the server with: cargo run --bin swisspipe${NC}"
    exit 1
fi

echo -e "${GREEN}Server is running ✓${NC}"
echo ""

# Create a temporary file for the test payload
TEMP_PAYLOAD=$(mktemp)
echo "${TEST_PAYLOAD}" > "${TEMP_PAYLOAD}"

# Run single performance test with 100 runs
echo -e "${BLUE}Running: Performance Test (100 runs)${NC}"
echo -e "Runs: 100, Warmup: 5"
echo ""

hyperfine \
    --runs 1000 \
    --warmup 5 \
    --export-json "performance_1000_runs.json" \
    --export-markdown "performance_1000_runs.md" \
    "curl -s -X POST '${FULL_URL}' \
     -H 'Content-Type: application/json' \
     -H 'Custom-Test-Header: performance-test' \
     -H 'User-Agent: SwissPipe-Performance-Test/1.0' \
     --data @${TEMP_PAYLOAD}"

echo ""
echo -e "${GREEN}Performance test completed${NC}"
echo ""

# Cleanup
rm -f "${TEMP_PAYLOAD}"

# Summary
echo -e "${BLUE}Performance Test Summary${NC}"
echo -e "${BLUE}========================${NC}"
echo ""
echo -e "${GREEN}✓ Performance Test (100 runs) completed${NC}"
echo ""
echo -e "${YELLOW}Results exported to:${NC}"
echo -e "  - performance_100_runs.json"
echo -e "  - performance_100_runs.md"
echo ""
echo -e "${BLUE}Performance testing completed successfully!${NC}"