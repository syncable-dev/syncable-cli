# 🚀 Turbo Security Analyzer

Ultra-fast security scanning that's 10-100x faster than traditional approaches.

## Overview

The Turbo Security Analyzer is a high-performance security scanner that utilizes Rust's full capabilities for blazing fast analysis. It achieves dramatic speedups through:

- **Smart File Selection**: Eliminates 80-90% of work upfront using gitignore-aware discovery
- **Multi-Pattern Matching**: Aho-Corasick algorithm for simultaneous pattern search  
- **Memory-Mapped I/O**: Zero-copy file reading for large files
- **Parallel Processing**: Work-stealing thread pool with early termination
- **Intelligent Caching**: Concurrent caching with LRU eviction
- **Specialized Scanners**: Optimized for common file types

## Key Features

### 🎯 Smart File Discovery
- Git-aware file discovery using `git ls-files`
- Automatically skips ignored files
- Prioritizes critical files (.env, configs, secrets)

### ⚡ High-Performance Scanning
- Aho-Corasick multi-pattern matching
- Memory-mapped I/O for large files
- Work-stealing parallelism across CPU cores
- Early termination on critical findings

### 🧠 Intelligent Detection
- Advanced false positive reduction
- Context-aware confidence scoring
- GitIgnore risk assessment
- Template/example file exclusion

## Usage

### Integration with CLI

The turbo analyzer is integrated into the main security command:

```bash
# Fast security scan
sync-ctl security /path/to/project

# Include low severity findings (thorough mode)
sync-ctl security --include-low /path/to/project

# Skip secret detection (lightning mode)
sync-ctl security --no-secrets /path/to/project
```

### Scan Modes

The analyzer automatically chooses the best mode based on your flags:

- **Lightning**: Critical files only (.env, configs), basic patterns
- **Fast**: Smart sampling, priority patterns, skip large files
- **Balanced**: Good coverage with performance optimizations (default)
- **Thorough**: Full scan with all patterns (still optimized)
- **Paranoid**: Everything including low-severity findings

## Architecture

### Core Components

```
┌─────────────────────┐
│  File Discovery     │ ← Git-aware, smart filtering
└──────────┬──────────┘
           │
┌──────────▼──────────┐
│ Priority Scoring    │ ← Critical files first
└──────────┬──────────┘
           │
┌──────────▼──────────┐
│  Pattern Engine     │ ← Aho-Corasick matching
└──────────┬──────────┘
           │
┌──────────▼──────────┐
│  Parallel Scanner   │ ← Work-stealing threads
└──────────┬──────────┘
           │
┌──────────▼──────────┐
│   Result Cache      │ ← Concurrent caching
└──────────┬──────────┘
           │
┌──────────▼──────────┐
│  Report Generator   │ ← Aggregation & scoring
└─────────────────────┘
```

### Pattern Categories

- **Secrets**: API keys, passwords, tokens
- **Environment Variables**: Sensitive config values
- **Cryptographic Material**: Private keys, certificates
- **Cloud Credentials**: AWS, GCP, Azure keys
- **Database Connections**: Connection strings with credentials

## Performance

Typical performance improvements over traditional scanning:

- **Lightning Mode**: 50-100x faster (critical files only)
- **Fast Mode**: 20-50x faster (smart sampling)
- **Balanced Mode**: 10-25x faster (default, good coverage)
- **Thorough Mode**: 5-10x faster (comprehensive scan)

## Implementation Details

### File Discovery Optimization

```rust
// Git-aware discovery (50x faster than walkdir)
git ls-files -z | parallel_process

// Smart filtering pipeline
files -> priority_score -> sort -> filter_by_mode
```

### Pattern Matching

```rust
// Aho-Corasick for multi-pattern search
let patterns = ["password", "api_key", "secret", ...];
let matcher = AhoCorasick::new(patterns);

// Single pass through content
for match in matcher.find_iter(content) {
    // Process match with confidence scoring
}
```

### Memory Mapping

```rust
// Zero-copy file reading for large files
let mmap = unsafe { MmapOptions::new().map(&file)? };
let content = simdutf8::from_utf8(&mmap)?;
```

### Concurrent Caching

```rust
// Thread-safe cache with DashMap
cache: DashMap<PathBuf, CachedResult>

// LRU eviction when reaching size limit
if size > limit * 0.9 {
    evict_least_recently_used();
}
```

## Security Features

### GitIgnore Risk Assessment

The analyzer provides comprehensive gitignore status for all findings:

- **TRACKED**: File is tracked by git (CRITICAL RISK)
- **EXPOSED**: File contains secrets but not in .gitignore (HIGH RISK)
- **PROTECTED**: File is properly ignored (GOOD)
- **SAFE**: File appears safe for version control

### False Positive Reduction

Advanced techniques to minimize false positives:

- Skip documentation and comment lines
- Exclude template/example files
- Ignore placeholder values
- Context-aware confidence scoring

## Contributing

The turbo analyzer is designed for extensibility:

- Add new pattern sets in `pattern_engine.rs`
- Extend file discovery logic in `file_discovery.rs`
- Implement additional scanners in `scanner.rs`

## License

Same as the parent project. 