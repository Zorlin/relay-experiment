#!/bin/bash
# Script to test connecting to a relay via DNS hostname

# Default hostname to test or use from command line
HOSTNAME=${1:-relay01.lon.riff.cc}

echo "Testing connection to $HOSTNAME..."

# Create/update .env file with the domain
echo "DOMAINE=$HOSTNAME" > .env.test
# Set env vars to ensure DNS is preserved
echo "LIBP2P_WEBSOCKET_PRESERVE_DNS=true" >> .env.test 

# Set env vars for the test
export RUST_LOG=info
export DOTENV_FILENAME=.env.test
export TEST_RELAY=$HOSTNAME

echo "📝 Using domain $HOSTNAME for connection test"
echo "👉 CRITICAL: Ensuring DNS hostnames are preserved for TLS validation"

# Run the test with RUST_BACKTRACE to see better errors
RUST_BACKTRACE=1 cargo run

# Check exit status
if [ $? -eq 0 ]; then
  echo "✅ Connection test to $HOSTNAME succeeded!"
else
  echo "❌ Connection test to $HOSTNAME failed!"
  exit 1
fi 