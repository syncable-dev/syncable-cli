pub mod rust;
pub mod javascript;
pub mod python;
pub mod go;
pub mod java;

use crate::analyzer::{DetectedTechnology, DetectedLanguage};
use crate::error::Result;
use std::collections::HashMap;

/// Common interface for language-specific framework detection
pub trait LanguageFrameworkDetector {
    /// Detect frameworks for a specific language
    fn detect_frameworks(&self, language: &DetectedLanguage) -> Result<Vec<DetectedTechnology>>;
    
    /// Get the supported language name(s) for this detector
    fn supported_languages(&self) -> Vec<&'static str>;
}

/// Technology detection rules with proper classification and relationships
#[derive(Clone, Debug)]
pub struct TechnologyRule {
    pub name: String,
    pub category: crate::analyzer::TechnologyCategory,
    pub confidence: f32,
    pub dependency_patterns: Vec<String>,
    /// Dependencies this technology requires (e.g., Next.js requires React)
    pub requires: Vec<String>,
    /// Technologies that conflict with this one (mutually exclusive)
    pub conflicts_with: Vec<String>,
    /// Whether this technology typically drives the architecture
    pub is_primary_indicator: bool,
    /// Alternative names for this technology
    pub alternative_names: Vec<String>,
}

/// Shared utilities for framework detection across languages
pub struct FrameworkDetectionUtils;

impl FrameworkDetectionUtils {
    /// Generic technology detection based on dependency patterns
    pub fn detect_technologies_by_dependencies(
        rules: &[TechnologyRule],
        dependencies: &[String],
        base_confidence: f32,
    ) -> Vec<DetectedTechnology> {
        let mut technologies = Vec::new();
        
        // Debug logging for Tanstack Start detection
        let tanstack_deps: Vec<_> = dependencies.iter()
            .filter(|dep| dep.contains("tanstack") || dep.contains("vinxi"))
            .collect();
        if !tanstack_deps.is_empty() {
            log::debug!("Found potential Tanstack dependencies: {:?}", tanstack_deps);
        }
        
        for rule in rules {
            let mut matches = 0;
            let total_patterns = rule.dependency_patterns.len();
            
            if total_patterns == 0 {
                continue;
            }
            
            for pattern in &rule.dependency_patterns {
                let matching_deps: Vec<_> = dependencies.iter()
                    .filter(|dep| Self::matches_pattern(dep, pattern))
                    .collect();
                    
                if !matching_deps.is_empty() {
                    matches += 1;
                    
                    // Debug logging for Tanstack Start specifically
                    if rule.name.contains("Tanstack") {
                        log::debug!("Tanstack Start: Pattern '{}' matched dependencies: {:?}", pattern, matching_deps);
                    }
                }
            }
            
            // Calculate confidence based on pattern matches and base language confidence
            if matches > 0 {
                let pattern_confidence = matches as f32 / total_patterns as f32;
                // Use additive approach instead of multiplicative to avoid extremely low scores
                // Base confidence provides a floor, pattern confidence provides the scaling
                let final_confidence = (rule.confidence * pattern_confidence + base_confidence * 0.1).min(1.0);
                
                // Debug logging for Tanstack Start detection
                if rule.name.contains("Tanstack") {
                    log::debug!("Tanstack Start detected with {} matches out of {} patterns, confidence: {:.2}", 
                              matches, total_patterns, final_confidence);
                }
                
                technologies.push(DetectedTechnology {
                    name: rule.name.clone(),
                    version: None, // TODO: Extract version from dependencies
                    category: rule.category.clone(),
                    confidence: final_confidence,
                    requires: rule.requires.clone(),
                    conflicts_with: rule.conflicts_with.clone(),
                    is_primary: rule.is_primary_indicator,
                });
            } else if rule.name.contains("Tanstack") {
                // Debug logging when Tanstack Start is not detected
                log::debug!("Tanstack Start not detected - no patterns matched. Available dependencies: {:?}", 
                          dependencies.iter().take(10).collect::<Vec<_>>());
            }
        }
        
        technologies
    }

    /// Check if a dependency matches a pattern (supports wildcards)
    pub fn matches_pattern(dependency: &str, pattern: &str) -> bool {
        if pattern.contains('*') {
            // Simple wildcard matching
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                dependency.starts_with(parts[0]) && dependency.ends_with(parts[1])
            } else {
                dependency.contains(&pattern.replace('*', ""))
            }
        } else {
            dependency == pattern || dependency.contains(pattern)
        }
    }

    /// Resolves conflicts between mutually exclusive technologies
    pub fn resolve_technology_conflicts(technologies: Vec<DetectedTechnology>) -> Vec<DetectedTechnology> {
        let mut resolved = Vec::new();
        let mut name_to_tech: HashMap<String, DetectedTechnology> = HashMap::new();
        
        // First pass: collect all technologies
        for tech in technologies {
            if let Some(existing) = name_to_tech.get(&tech.name) {
                // Keep the one with higher confidence
                if tech.confidence > existing.confidence {
                    name_to_tech.insert(tech.name.clone(), tech);
                }
            } else {
                name_to_tech.insert(tech.name.clone(), tech);
            }
        }
        
        // Second pass: resolve conflicts
        let all_techs: Vec<_> = name_to_tech.values().collect();
        let mut excluded_names = std::collections::HashSet::new();
        
        for tech in &all_techs {
            if excluded_names.contains(&tech.name) {
                continue;
            }
            
            // Check for conflicts
            for conflict in &tech.conflicts_with {
                if let Some(conflicting_tech) = name_to_tech.get(conflict) {
                    if tech.confidence > conflicting_tech.confidence {
                        excluded_names.insert(conflict.clone());
                        log::info!("Excluding {} (confidence: {}) in favor of {} (confidence: {})", 
                                  conflict, conflicting_tech.confidence, tech.name, tech.confidence);
                    } else {
                        excluded_names.insert(tech.name.clone());
                        log::info!("Excluding {} (confidence: {}) in favor of {} (confidence: {})", 
                                  tech.name, tech.confidence, conflict, conflicting_tech.confidence);
                        break;
                    }
                }
            }
        }
        
        // Collect non-excluded technologies
        for tech in name_to_tech.into_values() {
            if !excluded_names.contains(&tech.name) {
                resolved.push(tech);
            }
        }
        
        resolved
    }

    /// Marks technologies that are primary drivers of the application architecture
    pub fn mark_primary_technologies(mut technologies: Vec<DetectedTechnology>) -> Vec<DetectedTechnology> {
        use crate::analyzer::TechnologyCategory;
        
        // Meta-frameworks are always primary
        let mut has_meta_framework = false;
        for tech in &mut technologies {
            if matches!(tech.category, TechnologyCategory::MetaFramework) {
                tech.is_primary = true;
                has_meta_framework = true;
            }
        }
        
        // If no meta-framework, mark the highest confidence backend or frontend framework as primary
        if !has_meta_framework {
            let mut best_framework: Option<usize> = None;
            let mut best_confidence = 0.0;
            
            for (i, tech) in technologies.iter().enumerate() {
                if matches!(tech.category, TechnologyCategory::BackendFramework | TechnologyCategory::FrontendFramework) {
                    if tech.confidence > best_confidence {
                        best_confidence = tech.confidence;
                        best_framework = Some(i);
                    }
                }
            }
            
            if let Some(index) = best_framework {
                technologies[index].is_primary = true;
            }
        }
        
        technologies
    }
} 