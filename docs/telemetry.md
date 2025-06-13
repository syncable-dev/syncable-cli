# Syncable CLI Telemetry

The Syncable CLI includes optional, anonymized telemetry to help us understand how the tool is being used and where to focus our development efforts. This document explains what data is collected, how we protect your privacy, and how to control telemetry.

## 🔒 Privacy First

**Your privacy is our top priority.** All telemetry data is:

- **Completely anonymized** - No personal information is ever collected
- **No code content** - We never see your source code, file names, or paths
- **No sensitive data** - Environment variables, secrets, and credentials are never transmitted
- **Opt-out friendly** - Easy to disable with multiple methods

## 📊 What We Collect

### Command Usage Metrics
- Command name (analyze, generate, etc.)
- Execution duration 
- Success/failure status

### Project Analysis Metrics
- Project type (single project vs monorepo)
- File count (bucketed as small/medium/large/xlarge for privacy)
- Number of detected languages
- Number of detected frameworks
- Analysis duration

### Generation Metrics
- Generation type (dockerfile, compose, terraform)
- Generated file size
- Generation duration
- Success/failure status

### Error Metrics
- Error type (anonymized)
- Component where error occurred

### Technical Identifiers
- **Session ID**: Random UUID generated per CLI execution
- **Install ID**: Random UUID generated per installation
- **CLI version**: The version of syncable-cli you're using

## ❌ What We DON'T Collect

- **Source code** or file contents
- **File names** or directory paths
- **Environment variables** or configuration values
- **Personal information** (usernames, email addresses, etc.)
- **Network information** (IP addresses, hostnames, etc.)
- **Project-specific data** (exact file counts, dependency names, etc.)

## 🎯 Why We Collect This Data

The anonymized usage data helps us:

- **Understand feature usage** - Which commands and features are most popular
- **Identify performance issues** - Where the CLI is slow or inefficient
- **Detect error patterns** - Common issues users encounter
- **Guide development priorities** - What to work on next
- **Improve user experience** - Make the tool faster and more reliable

## 🔧 Controlling Telemetry

### Check Current Status

```bash
sync-ctl telemetry status
```

This shows whether telemetry is enabled and displays configuration details.

### View Detailed Information

```bash
sync-ctl telemetry info
```

Shows exactly what data is collected and our privacy policy.

### Opt Out (Disable Telemetry)

```bash
sync-ctl telemetry opt-out
```

This creates a file at `~/.syncable-cli/telemetry-opt-out` that disables telemetry.

### Opt In (Enable Telemetry)

```bash
sync-ctl telemetry opt-in
```

Removes the opt-out file to re-enable telemetry.

### Environment Variable Control

Set this environment variable to disable telemetry:

```bash
export SYNCABLE_TELEMETRY_ENABLED=false
```

Add this to your shell profile (`.bashrc`, `.zshrc`, etc.) to make it permanent.

## 🛡️ Data Security

- **Encrypted transmission** - All data is sent over HTTPS
- **No data retention** - We don't store raw telemetry data long-term
- **Aggregated analysis** - Data is only used in aggregate, never individually
- **Open source** - The telemetry code is open source and auditable

## 🌐 Data Transmission

Telemetry data is sent using the OpenTelemetry standard to our telemetry endpoint at `https://telemetry.syncable.dev`. The transmission is:

- **Non-blocking** - Never slows down CLI execution
- **Fault-tolerant** - CLI works normally even if telemetry fails
- **Minimal overhead** - Designed to have zero impact on performance

## 📋 Examples

### Typical Telemetry Event

Here's an example of what a telemetry event looks like (in JSON format for clarity):

```json
{
  "timestamp": 1640995200,
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "install_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
  "metric_type": "command_executed",
  "value": 1,
  "labels": {
    "command": "analyze",
    "success": "true"
  }
}
```

Notice how there's no personal information, file paths, or code content.

### Project Analysis Event

```json
{
  "timestamp": 1640995201,
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "install_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
  "metric_type": "project_analyzed",
  "value": 1,
  "labels": {
    "project_type": "single_project",
    "file_count_bucket": "medium"
  }
}
```

File counts are bucketed (small/medium/large/xlarge) to protect privacy while still providing useful data.

## 🤝 Transparency

We believe in complete transparency about data collection:

- This documentation explains exactly what we collect
- The telemetry code is open source in this repository
- You can audit the code to verify our claims
- Easy opt-out options are always available

## 📞 Questions or Concerns

If you have questions about telemetry or privacy:

- **Open an issue** on GitHub
- **Review the code** in `src/monitoring/`
- **Check our privacy policy** at https://syncable.dev/privacy
- **Contact us** at privacy@syncable.dev

## 🔄 Future Changes

If we ever change what telemetry data is collected:

- **Major version bump** - Breaking changes will increment the major version
- **Documentation updates** - This document will be updated
- **Announcement** - Changes will be announced in release notes
- **Opt-in required** - New data types will require explicit opt-in

Thank you for helping us improve Syncable CLI! 🚀 