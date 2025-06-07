#!/bin/bash

echo "🧪 Testing Syncable CLI Smart Update Check"
echo "==========================================="

# Test 1: Clear cache and check with debug
echo -e "\n📋 Test 1: Clear cache and check with debug mode"
SYNC_CTL_DEBUG=1 cargo run -- --clear-update-cache analyze . 2>&1 | grep -E "(Checking for updates|Current version|Latest version|Update check skipped|Update available in cache)"

# Test 2: Check if intelligent cache works
echo -e "\n📋 Test 2: Second run should use smart cache (2-hour window)"
sleep 1
SYNC_CTL_DEBUG=1 cargo run -- analyze . 2>&1 | grep -E "(Update check skipped|Checking for updates|Update available in cache)"

# Test 3: Show cache contents
echo -e "\n📋 Test 3: Examining cache contents"
if [[ "$OSTYPE" == "darwin"* ]]; then
    CACHE_FILE="$HOME/Library/Caches/syncable-cli/version_cache.json"
else
    CACHE_FILE="$HOME/.cache/syncable-cli/version_cache.json"
fi

if [ -f "$CACHE_FILE" ]; then
    echo "Cache file found at: $CACHE_FILE"
    echo "Cache contents:"
    cat "$CACHE_FILE" | jq . 2>/dev/null || cat "$CACHE_FILE"
else
    echo "No cache file found at: $CACHE_FILE"
fi

# Test 4: Force check again
echo -e "\n📋 Test 4: Force check with --clear-update-cache"
SYNC_CTL_DEBUG=1 cargo run -- --clear-update-cache analyze . 2>&1 | grep -E "(Update cache cleared|Checking for updates|Removed update cache)"

echo -e "\n✅ Test complete!"
echo "Smart update system features:"
echo "  • Checks every 2 hours when no update available"
echo "  • Shows update immediately if cached version is newer" 
echo "  • Stores detailed version info in JSON cache"
echo "  • Enhanced notification with clear update instructions"
echo "  • Multiple update methods (Cargo, direct download, install script)"
echo "  • To test with a real update notification, the GitHub release needs to have a newer version than 0.5.0" 