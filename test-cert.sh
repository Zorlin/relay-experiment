#!/bin/bash
# Script to test TLS certificate validity using curl

HOSTNAME=${1:-relay01.lon.riff.cc}
echo "Testing TLS certificate for $HOSTNAME..."

# First try with hostname
echo "Connecting via hostname..."
curl -v "https://$HOSTNAME" 2>&1 | grep -A 10 "Server certificate"

# Resolve IP and test with IP directly
IP=$(dig +short $HOSTNAME)
echo -e "\nIP address: $IP"

# Now try with IP but send proper hostname via SNI
echo -e "\nConnecting via IP with SNI hostname..."
curl -v --resolve "$HOSTNAME:443:$IP" "https://$HOSTNAME" 2>&1 | grep -A 10 "Server certificate"

echo -e "\nThe issue is likely that libp2p resolves the DNS name to an IP,
but when it makes the TLS connection, it's validating against the IP 
instead of preserving the hostname for TLS certificate validation." 