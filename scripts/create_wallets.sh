#!/bin/bash

bitcoincli=$(command -v bitcoin-cli)
opts=( -rpcuser="testuser" -rpcpassword="lq6zequb-gYTdF2_ZEUtr8ywTXzLYtknzWU4nV8uVoo=" -regtest -named)

# Unload wallets if they exist
$bitcoincli "${opts[@]}" unloadwallet wallet_name="alice" 2>/dev/null || true
$bitcoincli "${opts[@]}" unloadwallet wallet_name="bob" 2>/dev/null || true

# Remove existing wallet directories
rm -rf /home/bitcoin/.bitcoin/regtest/wallets/alice
rm -rf /home/bitcoin/.bitcoin/regtest/wallets/bob

# Create fresh wallets
$bitcoincli "${opts[@]}" createwallet wallet_name="alice" descriptors="false"
$bitcoincli "${opts[@]}" createwallet wallet_name="bob" descriptors="false"

# Generate addresses and mine blocks
aliceAddress=$($bitcoincli "${opts[@]}" -rpcwallet=alice getnewaddress bec32)
$bitcoincli "${opts[@]}" generatetoaddress 101 ${aliceAddress} &> /dev/null
bobAddress=$($bitcoincli "${opts[@]}" -rpcwallet=bob getnewaddress bec32)
$bitcoincli "${opts[@]}" generatetoaddress 201 ${bobAddress} &> /dev/null

echo "Alice's address: ${aliceAddress}"
echo "Bob's address: ${bobAddress}"