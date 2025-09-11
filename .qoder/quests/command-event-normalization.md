# Command Event Normalization Design

## Summary

This document outlines the changes needed to fix the duplicate telemetry events issue in the syncable-cli application. Currently, the `security` and `vulnerabilities` commands each generate two telemetry events, causing data duplication. The solution involves modifying the telemetry client to use descriptive event names directly and removing the duplicate event calls in the command handlers.

## Problem Statement

When running commands like `sync-ctl security .`, two events are generated:
- "security" event with properties


- "Security Scan" event

Similarly for `sync-ctl vulnerabilities .`:
- "vulnerabilities" event with properties


- "Vulnerability Scan" event

This duplication creates unnecessary noise in telemetry data and can skew analytics.

## Solution

The solution involves two key changes:

1. Modify the telemetry client methods to directly use the descriptive event names:
   - `track_security()` will track "Security Scan" events


   - `track_vulnerabilities()` will track "Vulnerability Scan" events

2. Remove the duplicate event calls in the command handlers:
   - Remove `track_security_scan()` call from the security command handler


   - Remove `track_vulnerability_scan()` call from the vulnerabilities command handler

## Implementation Details

### File: src/telemetry/client.rs

Update the `track_security` method to use the descriptive event name:
```rust

pub fn track_security(&self, properties: HashMap<String, serde_json::Value>) {
    self.track_event("Security Scan", properties);
}
```

Update the `track_vulnerabilities` method to use the descriptive event name:
```rust

pub fn track_vulnerabilities(&self, properties: HashMap<String, serde_json::Value>) {
    self.track_event("Vulnerability Scan", properties);
}
```

Update the deprecated methods to be no-ops with deprecation comments:
```rust


pub fn track_security_scan(&self) {
    // Deprecated: Use track_security with properties instead


}

pub fn track_vulnerability_scan(&self) {
    // Deprecated: Use track_vulnerabilities with properties instead


}
```

### File: src/main.rs

In the Security command handler, remove the duplicate event call:
```rust


// Remove this duplicate call


// if let Some(telemetry_client) = telemetry::get_telemetry_client() {
//     telemetry_client.track_security_scan();
// }
```

In the Vulnerabilities command handler, remove the duplicate event call:
```rust


// Remove this duplicate call


// if let Some(telemetry_client) = telemetry::get_telemetry_client() {
//     telemetry_client.track_vulnerability_scan();
// }
```

## Benefits

1. **Eliminates Duplicate Events**: Each command will generate exactly one telemetry event


2. **Maintains Event Properties**: All existing properties will still be captured


3. **Consistent Naming**: Event names will clearly indicate the type of scan performed


4. **Backward Compatibility**: Existing telemetry infrastructure remains unchanged


5. **Cleaner Analytics**: Reduces noise in telemetry data, making analysis more accurate

