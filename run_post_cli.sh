#!/usr/bin/env bash
# POST Payload Benchmark CLI
# Usage: ./run_post_cli.sh <payload_file> <duration>
# Example: ./run_post_cli.sh tests/test_post/payload_1kb.json 3

set -e

PAYLOAD_FILE="${1:-tests/test_post/payload_1kb.json}"
DURATION="${2:-3}"

if [ ! -f "$PAYLOAD_FILE" ]; then
    echo "Error: File not found: $PAYLOAD_FILE"
    echo "Available payload files:"
    ls -la tests/test_post/payload_*.json
    exit 1
fi

PAYLOAD=$(cat "$PAYLOAD_FILE")
PAYLOAD_SIZE=${#PAYLOAD}

echo "================================================================================"
echo "POST Benchmark: $PAYLOAD_FILE ($PAYLOAD_SIZE bytes)"
echo "================================================================================"

echo ""
echo "Running RequestX benchmark..."
uv run http-benchmark \
    --url "http://localhost/post" \
    --method "POST" \
    --body "$PAYLOAD" \
    --headers '{"Content-Type": "application/json"}' \
    --client "requestx" \
    --concurrency 1 \
    --duration "$DURATION"

echo ""
echo "Running requests benchmark..."
uv run http-benchmark \
    --url "http://localhost/post" \
    --method "POST" \
    --body "$PAYLOAD" \
    --headers '{"Content-Type": "application/json"}' \
    --client "requests" \
    --concurrency 1 \
    --duration "$DURATION"
