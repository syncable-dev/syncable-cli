# Analyze Folder Command Telemetry Design

## Overview

This document outlines the design for improving the telemetry events for the `analyze` command in the syncable-cli application. Currently, the command generates two separate telemetry events when executed, which is incorrect. The goal is to generate only one event per command execution while still capturing the different modes of operation (detailed view, JSON output, etc.).

## Current Issues

1. **Duplicate Events**: When running `sync-ctl analyze .`, two events are generated:
   - A generic "analyze" event
   - A specific "Analyze Folder" event

2. **Lack of Differentiation**: The current implementation doesn't capture how the analysis was performed (JSON output, detailed view, matrix view, etc.)

## Proposed Solution

Replace the two separate events with a single "Analyze Folder" event that includes properties to differentiate the analysis mode.

## Architecture

### Event Structure

The new telemetry event will have the following structure:

Event Name: "Analyze Folder"
Properties:
- analysis_mode: string (one of: "json", "detailed", "matrix", "summary")
- color_scheme: string (one of: "auto", "dark", "light")
- only_filter: string[] (list of filtered analysis aspects)

### Implementation Plan

1. **Remove duplicate event calls**: Eliminate the separate `track_analyze()` call
2. **Enhance the `track_analyze_folder()` method**: Add parameters to capture analysis mode
3. **Modify the main function**: Pass analysis parameters to the telemetry event
4. **Update the telemetry client**: Modify the `track_analyze_folder()` method to accept and process these parameters

## Detailed Design

### 1. Telemetry Client Modifications

The `TelemetryClient` struct will be updated to accept properties in the `track_analyze_folder` method:

Method signature:
- Current: `track_analyze_folder(&self)`
- New: `track_analyze_folder(&self, properties: HashMap<String, serde_json::Value>)`

Implementation:
- The method will pass the properties to the track_event function
- Properties will be merged with common properties before sending

### 2. Main Function Updates

In the main function, the analyze command handling will be modified:

Process for determining analysis mode:
- If json flag is true → "json"
- Else if detailed flag is true → "detailed"
- Else based on display option:
  - Matrix or None → "matrix"
  - Detailed → "detailed"
  - Summary → "summary"

Properties to capture:
- Analysis mode (determined by command flags)
- Color scheme (if specified)
- Only filter (if specified)

### 3. Remove Duplicate Event

The separate `telemetry_client.track_analyze()` call will be removed from the analyze command handling.

## Data Flow

``mermaid
graph TD
    A[User runs analyze command] --> B[CLI Parser]
    B --> C[Main Function]
    C --> D[Create telemetry properties]
    D --> E[Track single Analyze Folder event]
    E --> F[Send to PostHog]
```

## Benefits

1. **Single Event Per Command**: Only one telemetry event will be generated per analyze command execution
2. **Mode Differentiation**: The analysis mode (JSON, detailed, matrix, summary) will be captured in event properties
3. **Enhanced Analytics**: Better data for understanding how users interact with the analyze command
4. **Consistency**: Aligns with the pattern used for other commands like security scans

## Implementation Steps

1. Modify the `track_analyze_folder` method in the telemetry client to accept properties
2. Update the analyze command handling in main.rs to:
   - Remove the duplicate `track_analyze()` call
   - Create properties map with analysis mode and other relevant information
   - Call `track_analyze_folder` with the properties
3. Test the implementation to ensure only one event is generated with correct properties
4. Update any related tests

## Testing Plan

1. **Unit Tests**: Update telemetry tests to reflect the new method signature
2. **Integration Tests**: Verify that only one event is generated when running the analyze command
3. **Property Validation**: Confirm that the correct analysis mode is captured in event properties
4. **Edge Cases**: Test with various combinations of command-line options

## Backward Compatibility

This change is backward compatible with existing telemetry infrastructure. The event name remains "Analyze Folder", and the core telemetry collection mechanism is unchanged. The only difference is in the data captured with the event.

## Future Enhancements

1. **Performance Metrics**: Add analysis duration and file count to the telemetry properties
2. **Project Type Detection**: Include detected project types in the event properties
3. **Error Tracking**: Add success/failure status to the events
