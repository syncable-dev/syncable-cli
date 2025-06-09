//! # Cache Module
//! 
//! High-performance caching for security scan results using DashMap and blake3.

use std::path::PathBuf;
use std::time::{SystemTime, Duration};
use std::sync::Arc;

use dashmap::DashMap;

use log::{debug, trace};

use crate::analyzer::security::SecurityFinding;

/// Cache key for file content
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct CacheKey {
    pub file_path: PathBuf,
}

/// Cached scan result
#[derive(Debug, Clone)]
pub struct CachedResult {
    pub findings: Vec<SecurityFinding>,
    pub cached_at: SystemTime,
    pub access_count: u32,
}

/// High-performance security cache
pub struct SecurityCache {
    // Main cache storage
    cache: Arc<DashMap<PathBuf, CachedEntry, ahash::RandomState>>,
    
    // Cache configuration
    max_size_bytes: usize,
    current_size_bytes: Arc<parking_lot::Mutex<usize>>,
    eviction_threshold: f64,
    
    // Statistics
    hits: Arc<parking_lot::Mutex<u64>>,
    misses: Arc<parking_lot::Mutex<u64>>,
}

/// Internal cache entry
#[derive(Debug, Clone)]
struct CachedEntry {
    key: CacheKey,
    result: CachedResult,
    size_bytes: usize,
    last_accessed: SystemTime,
}

impl SecurityCache {
    /// Create a new cache with specified size in MB
    pub fn new(size_mb: usize) -> Self {
        let max_size_bytes = size_mb * 1024 * 1024;
        let hasher = ahash::RandomState::new();
        
        Self {
            cache: Arc::new(DashMap::with_hasher(hasher)),
            max_size_bytes,
            current_size_bytes: Arc::new(parking_lot::Mutex::new(0)),
            eviction_threshold: 0.9, // Start eviction at 90% capacity
            hits: Arc::new(parking_lot::Mutex::new(0)),
            misses: Arc::new(parking_lot::Mutex::new(0)),
        }
    }
    
    /// Get cached result for a file
    pub fn get(&self, file_path: &PathBuf) -> Option<Vec<SecurityFinding>> {
        let entry = self.cache.get_mut(file_path)?;
        
        // Update access statistics
        let mut entry = entry;
        entry.last_accessed = SystemTime::now();
        entry.result.access_count += 1;
        
        *self.hits.lock() += 1;
        trace!("Cache hit for: {}", file_path.display());
        
        Some(entry.result.findings.clone())
    }
    
    /// Insert a scan result into cache
    pub fn insert(&self, file_path: PathBuf, findings: Vec<SecurityFinding>) {
        // Calculate entry size
        let size_bytes = Self::estimate_size(&findings);
        
        // Check if we need to evict entries
        let current_size = *self.current_size_bytes.lock();
        if current_size + size_bytes > (self.max_size_bytes as f64 * self.eviction_threshold) as usize {
            self.evict_lru();
        }
        
        // Create cache key
        let key = CacheKey {
            file_path: file_path.clone(),
        };
        
        // Create cache entry
        let entry = CachedEntry {
            key,
            result: CachedResult {
                findings,
                cached_at: SystemTime::now(),
                access_count: 1,
            },
            size_bytes,
            last_accessed: SystemTime::now(),
        };
        
        // Insert into cache
        if let Some(old_entry) = self.cache.insert(file_path, entry) {
            // Subtract old entry size
            *self.current_size_bytes.lock() -= old_entry.size_bytes;
        }
        
        // Add new entry size
        *self.current_size_bytes.lock() += size_bytes;
        
        debug!("Cached result, current size: {} MB", 
               *self.current_size_bytes.lock() / (1024 * 1024));
    }
    
    /// Clear the entire cache
    pub fn clear(&self) {
        self.cache.clear();
        *self.current_size_bytes.lock() = 0;
        *self.hits.lock() = 0;
        *self.misses.lock() = 0;
        debug!("Cache cleared");
    }
    
    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let hits = *self.hits.lock();
        let misses = *self.misses.lock();
        let total = hits + misses;
        
        CacheStats {
            hits,
            misses,
            hit_rate: if total > 0 { hits as f64 / total as f64 } else { 0.0 },
            entries: self.cache.len(),
            size_bytes: *self.current_size_bytes.lock(),
            capacity_bytes: self.max_size_bytes,
        }
    }
    
    /// Evict least recently used entries
    fn evict_lru(&self) {
        let target_size = (self.max_size_bytes as f64 * 0.7) as usize; // Evict to 70% capacity
        let mut entries_to_remove = Vec::new();
        
        // Collect entries sorted by last access time
        let mut entries: Vec<(PathBuf, SystemTime, usize)> = self.cache.iter()
            .map(|entry| (entry.key().clone(), entry.last_accessed, entry.size_bytes))
            .collect();
        
        // Sort by last accessed (oldest first)
        entries.sort_by_key(|(_, last_accessed, _)| *last_accessed);
        
        // Determine which entries to remove
        let mut current_size = *self.current_size_bytes.lock();
        for (path, _, size) in entries {
            if current_size <= target_size {
                break;
            }
            
            entries_to_remove.push(path);
            current_size -= size;
        }
        
        // Count entries to remove
        let entries_removed = entries_to_remove.len();
        
        // Remove entries
        for path in entries_to_remove {
            if let Some((_, entry)) = self.cache.remove(&path) {
                *self.current_size_bytes.lock() -= entry.size_bytes;
            }
        }
        
        debug!("Evicted {} entries, new size: {} MB", 
               entries_removed,
               *self.current_size_bytes.lock() / (1024 * 1024));
    }
    

    
    /// Estimate memory size of findings
    fn estimate_size(findings: &[SecurityFinding]) -> usize {
        // Base size for the vector
        let mut size = std::mem::size_of::<Vec<SecurityFinding>>();
        
        // Add size for each finding
        for finding in findings {
            size += std::mem::size_of::<SecurityFinding>();
            
            // Add string sizes
            size += finding.id.len();
            size += finding.title.len();
            size += finding.description.len();
            
            if let Some(ref path) = finding.file_path {
                size += path.to_string_lossy().len();
            }
            
            if let Some(ref evidence) = finding.evidence {
                size += evidence.len();
            }
            
            // Add vector sizes
            size += finding.remediation.iter().map(|s| s.len()).sum::<usize>();
            size += finding.references.iter().map(|s| s.len()).sum::<usize>();
            size += finding.compliance_frameworks.iter().map(|s| s.len()).sum::<usize>();
            
            if let Some(ref cwe) = finding.cwe_id {
                size += cwe.len();
            }
        }
        
        size
    }
    
    /// Invalidate cache entries older than duration
    pub fn invalidate_older_than(&self, duration: Duration) {
        let cutoff = SystemTime::now() - duration;
        let mut removed = 0;
        
        self.cache.retain(|_, entry| {
            if entry.result.cached_at < cutoff {
                *self.current_size_bytes.lock() -= entry.size_bytes;
                removed += 1;
                false
            } else {
                true
            }
        });
        
        if removed > 0 {
            debug!("Invalidated {} stale cache entries", removed);
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub entries: usize,
    pub size_bytes: usize,
    pub capacity_bytes: usize,
}

impl CacheStats {
    /// Get human-readable size
    pub fn size_mb(&self) -> f64 {
        self.size_bytes as f64 / (1024.0 * 1024.0)
    }
    
    /// Get capacity utilization percentage
    pub fn utilization(&self) -> f64 {
        if self.capacity_bytes == 0 {
            0.0
        } else {
            (self.size_bytes as f64 / self.capacity_bytes as f64) * 100.0
        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::security::{SecuritySeverity, SecurityCategory};
    
    #[test]
    fn test_cache_basic_operations() {
        let cache = SecurityCache::new(10); // 10MB cache
        
        let path = PathBuf::from("/test/file.js");
        let findings = vec![
            SecurityFinding {
                id: "test-1".to_string(),
                title: "Test Finding".to_string(),
                description: "Test description".to_string(),
                severity: SecuritySeverity::High,
                category: SecurityCategory::SecretsExposure,
                file_path: Some(path.clone()),
                line_number: Some(10),
                column_number: Some(5),
                evidence: Some("evidence".to_string()),
                remediation: vec!["Fix it".to_string()],
                references: vec!["https://example.com".to_string()],
                cwe_id: Some("CWE-798".to_string()),
                compliance_frameworks: vec!["SOC2".to_string()],
            }
        ];
        
        // Test insert
        cache.insert(path.clone(), findings.clone());
        
        // Test get
        let cached = cache.get(&path);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().len(), 1);
        
        // Test stats
        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.entries, 1);
    }
    
    #[test]
    fn test_cache_eviction() {
        let cache = SecurityCache::new(1); // 1MB cache (small for testing)
        
        // Insert many entries to trigger eviction
        for i in 0..1000 {
            let path = PathBuf::from(format!("/test/file{}.js", i));
            let findings = vec![
                SecurityFinding {
                    id: format!("test-{}", i),
                    title: "Test Finding with very long title to consume memory".to_string(),
                    description: "Test description that is also quite long to use up cache space".to_string(),
                    severity: SecuritySeverity::High,
                    category: SecurityCategory::SecretsExposure,
                    file_path: Some(path.clone()),
                    line_number: Some(10),
                    column_number: Some(5),
                    evidence: Some("evidence with long content to test memory usage".to_string()),
                    remediation: vec!["Fix it with a long remediation message".to_string()],
                    references: vec!["https://example.com/very/long/url/path".to_string()],
                    cwe_id: Some("CWE-798".to_string()),
                    compliance_frameworks: vec!["SOC2".to_string(), "GDPR".to_string()],
                }
            ];
            
            cache.insert(path, findings);
        }
        
        // Cache should have evicted some entries
        let stats = cache.stats();
        assert!(stats.entries < 1000);
        assert!(stats.utilization() <= 90.0);
    }
    
    #[test]
    fn test_cache_invalidation() {
        let cache = SecurityCache::new(10);
        
        let path = PathBuf::from("/test/file.js");
        let findings = vec![];
        
        cache.insert(path.clone(), findings);
        
        // Invalidate entries older than 0 seconds (all entries)
        cache.invalidate_older_than(Duration::from_secs(0));
        
        // Cache should be empty
        assert!(cache.get(&path).is_none());
        assert_eq!(cache.stats().entries, 0);
    }
} 