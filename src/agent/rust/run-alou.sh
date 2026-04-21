#!/bin/bash
# run-alou.sh - Start Alou with proper environment variables

set -e

cd "$(dirname "$0")"

# Load environment variables from .env file
if [ -f .env ]; then
    echo "Loading environment variables from .env..."
    set -a
    source .env
    set +a
    echo "✅ Environment variables loaded"
else
    echo "⚠️  Warning: .env file not found"
fi

# Set API configuration for DeepSeek
export ANTHROPIC_API_KEY="sk-0e701b56fd2448b9b8c1b485486a2d23"
export ANTHROPIC_BASE_URL="https://api.deepseek.com"

echo "Starting Alou with DeepSeek model..."
echo "Wallet address: ${POLY_ADDRESS:-Not set}"
echo "RPC URL: ${RPC_URL:-Not set}"

# Run Alou with full access permissions
./target/release/alou --model deepseek-chat --permission-mode=danger-full-access