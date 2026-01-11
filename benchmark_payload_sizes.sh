#!/bin/bash

# Payload sizes to test
SIZES=("50B" "1KB" "10KB" "100KB" "512KB" "1MB" "2MB")

echo "=============================================="
echo "RequestX POST Benchmark - Payload Size Tests"
echo "=============================================="
echo "Configuration: POST, 1 concurrency, 3 seconds"
echo "=============================================="
echo ""

for SIZE in "${SIZES[@]}"; do
    echo ""
    echo "========================================"
    echo "Testing Payload: $SIZE"
    echo "========================================"
    
    PAYLOAD=$(cat "/tmp/payload_${SIZE}.json")
    
    echo "Running requestx benchmark..."
    RESULT1=$(uv run http-benchmark \
        --url "http://localhost/post" \
        --method POST \
        --body "$PAYLOAD" \
        --headers '{"Content-Type": "application/json"}' \
        --client requestx \
        --concurrency 1 \
        --duration 3 2>&1 | grep -A1 "Benchmark Results:")
    
    echo "Running requests benchmark..."
    RESULT2=$(uv run http-benchmark \
        --url "http://localhost/post" \
        --method POST \
        --body "$PAYLOAD" \
        --headers '{"Content-Type": "application/json"}' \
        --client requests \
        --concurrency 1 \
        --duration 3 2>&1 | grep -A1 "Benchmark Results:")
    
    echo "Results:"
    echo "  requestx: $RESULT1"
    echo "  requests: $RESULT2"
    
    sleep 1
done

echo ""
echo "=============================================="
echo "All tests completed!"
echo "=============================================="
