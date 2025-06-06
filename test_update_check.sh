#!/bin/bash

echo "🧪 Testing Syncable CLI Update Check"
echo "===================================="

# Test 1: Clear cache and check with debug
echo -e "\n📋 Test 1: Clear cache and check with debug mode"
SYNC_CTL_DEBUG=1 cargo run -- --clear-update-cache analyze . 2>&1 | grep -E "(Checking for updates|Current version|Latest version|Update check skipped)"

# Test 2: Check if cache works
echo -e "\n📋 Test 2: Second run should use cache"
sleep 1
SYNC_CTL_DEBUG=1 cargo run -- analyze . 2>&1 | grep -E "(Update check skipped|Checking for updates)"

# Test 3: Force check again
echo -e "\n📋 Test 3: Force check with --clear-update-cache"
SYNC_CTL_DEBUG=1 cargo run -- --clear-update-cache analyze . 2>&1 | grep -E "(Update cache cleared|Checking for updates)"

echo -e "\n✅ Test complete!"
echo "To test with a real update notification, the GitHub release needs to have a newer version than 0.4.1" 