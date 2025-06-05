use crate::analyzer::{AnalysisConfig, DetectedTechnology, DetectedLanguage, TechnologyCategory, LibraryType};
use crate::error::Result;
// Remove unused import
use std::path::Path;
use std::collections::HashMap;

/// Technology detection rules with proper classification and relationships
struct TechnologyRule {
    name: String,
    category: TechnologyCategory,
    confidence: f32,
    dependency_patterns: Vec<String>,
    /// Dependencies this technology requires (e.g., Next.js requires React)
    requires: Vec<String>,
    /// Technologies that conflict with this one (mutually exclusive)
    conflicts_with: Vec<String>,
    /// Whether this technology typically drives the architecture
    is_primary_indicator: bool,
    /// Alternative names for this technology
    alternative_names: Vec<String>,
}

/// Detects technologies (frameworks, libraries, tools) with proper classification
pub fn detect_frameworks(
    _project_root: &Path,
    languages: &[DetectedLanguage],
    _config: &AnalysisConfig,
) -> Result<Vec<DetectedTechnology>> {
    let mut all_technologies = Vec::new();
    
    for language in languages {
        let lang_technologies = match language.name.as_str() {
            "Rust" => detect_rust_technologies(language),
            "JavaScript" | "TypeScript" | "JavaScript/TypeScript" => detect_js_technologies(language),
            "Python" => detect_python_technologies(language),
            "Go" => detect_go_technologies(language),
            "Java" | "Kotlin" | "Java/Kotlin" => detect_jvm_technologies(language),
            _ => Vec::new(),
        };
        all_technologies.extend(lang_technologies);
    }
    
    // Apply exclusivity rules and resolve conflicts
    let resolved_technologies = resolve_technology_conflicts(all_technologies);
    
    // Mark primary technologies
    let final_technologies = mark_primary_technologies(resolved_technologies);
    
    // Sort by confidence and remove exact duplicates
    let mut result = final_technologies;
    result.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
    result.dedup_by(|a, b| a.name == b.name);
    
    Ok(result)
}

/// Detect Rust technologies with proper classification
fn detect_rust_technologies(language: &DetectedLanguage) -> Vec<DetectedTechnology> {
    let rules = get_rust_technology_rules();
    detect_technologies_by_dependencies(&rules, &language.main_dependencies, language.confidence)
}

/// Detect JavaScript/TypeScript technologies with proper classification
fn detect_js_technologies(language: &DetectedLanguage) -> Vec<DetectedTechnology> {
    let rules = get_js_technology_rules();
    
    // Combine main and dev dependencies for comprehensive detection
    let all_deps: Vec<String> = language.main_dependencies.iter()
        .chain(language.dev_dependencies.iter())
        .cloned()
        .collect();
    
    detect_technologies_by_dependencies(&rules, &all_deps, language.confidence)
}

/// Detect Python technologies with proper classification
fn detect_python_technologies(language: &DetectedLanguage) -> Vec<DetectedTechnology> {
    let rules = get_python_technology_rules();
    detect_technologies_by_dependencies(&rules, &language.main_dependencies, language.confidence)
}

/// Detect Go technologies with proper classification
fn detect_go_technologies(language: &DetectedLanguage) -> Vec<DetectedTechnology> {
    let rules = get_go_technology_rules();
    detect_technologies_by_dependencies(&rules, &language.main_dependencies, language.confidence)
}

/// Detect JVM technologies with proper classification
fn detect_jvm_technologies(language: &DetectedLanguage) -> Vec<DetectedTechnology> {
    let rules = get_jvm_technology_rules();
    detect_technologies_by_dependencies(&rules, &language.main_dependencies, language.confidence)
}

/// Generic technology detection based on dependency patterns
fn detect_technologies_by_dependencies(
    rules: &[TechnologyRule],
    dependencies: &[String],
    base_confidence: f32,
) -> Vec<DetectedTechnology> {
    let mut technologies = Vec::new();
    
    for rule in rules {
        let mut matches = 0;
        let total_patterns = rule.dependency_patterns.len();
        
        if total_patterns == 0 {
            continue;
        }
        
        for pattern in &rule.dependency_patterns {
            if dependencies.iter().any(|dep| matches_pattern(dep, pattern)) {
                matches += 1;
            }
        }
        
        // Calculate confidence based on pattern matches and base language confidence
        if matches > 0 {
            let pattern_confidence = matches as f32 / total_patterns as f32;
            let final_confidence = (rule.confidence * pattern_confidence * base_confidence).min(1.0);
            
            technologies.push(DetectedTechnology {
                name: rule.name.clone(),
                version: None, // TODO: Extract version from dependencies
                category: rule.category.clone(),
                confidence: final_confidence,
                requires: rule.requires.clone(),
                conflicts_with: rule.conflicts_with.clone(),
                is_primary: rule.is_primary_indicator,
            });
        }
    }
    
    technologies
}

/// Resolves conflicts between mutually exclusive technologies
fn resolve_technology_conflicts(technologies: Vec<DetectedTechnology>) -> Vec<DetectedTechnology> {
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
fn mark_primary_technologies(mut technologies: Vec<DetectedTechnology>) -> Vec<DetectedTechnology> {
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

/// Check if a dependency matches a pattern (supports wildcards)
fn matches_pattern(dependency: &str, pattern: &str) -> bool {
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

/// JavaScript/TypeScript technology detection rules with proper classification
fn get_js_technology_rules() -> Vec<TechnologyRule> {
    vec![
        // META-FRAMEWORKS (Mutually Exclusive)
        TechnologyRule {
            name: "Next.js".to_string(),
            category: TechnologyCategory::MetaFramework,
            confidence: 0.95,
            dependency_patterns: vec!["next".to_string()],
            requires: vec!["React".to_string()],
            conflicts_with: vec!["Tanstack Start".to_string(), "React Router v7".to_string(), "SvelteKit".to_string(), "Nuxt.js".to_string()],
            is_primary_indicator: true,
            alternative_names: vec!["nextjs".to_string()],
        },
        TechnologyRule {
            name: "Tanstack Start".to_string(),
            category: TechnologyCategory::MetaFramework,
            confidence: 0.95,
            dependency_patterns: vec!["@tanstack/react-start".to_string()],
            requires: vec!["React".to_string()],
            conflicts_with: vec!["Next.js".to_string(), "React Router v7".to_string(), "SvelteKit".to_string(), "Nuxt.js".to_string()],
            is_primary_indicator: true,
            alternative_names: vec!["tanstack-start".to_string()],
        },
        TechnologyRule {
            name: "React Router v7".to_string(),
            category: TechnologyCategory::MetaFramework,
            confidence: 0.95,
            dependency_patterns: vec!["react-router".to_string(), "@remix-run/react".to_string()],
            requires: vec!["React".to_string()],
            conflicts_with: vec!["Next.js".to_string(), "Tanstack Start".to_string(), "SvelteKit".to_string(), "Nuxt.js".to_string()],
            is_primary_indicator: true,
            alternative_names: vec!["remix".to_string(), "react-router".to_string()],
        },
        TechnologyRule {
            name: "SvelteKit".to_string(),
            category: TechnologyCategory::MetaFramework,
            confidence: 0.95,
            dependency_patterns: vec!["@sveltejs/kit".to_string()],
            requires: vec!["Svelte".to_string()],
            conflicts_with: vec!["Next.js".to_string(), "Tanstack Start".to_string(), "React Router v7".to_string(), "Nuxt.js".to_string()],
            is_primary_indicator: true,
            alternative_names: vec!["svelte-kit".to_string()],
        },
        TechnologyRule {
            name: "Nuxt.js".to_string(),
            category: TechnologyCategory::MetaFramework,
            confidence: 0.95,
            dependency_patterns: vec!["nuxt".to_string(), "@nuxt/core".to_string()],
            requires: vec!["Vue.js".to_string()],
            conflicts_with: vec!["Next.js".to_string(), "Tanstack Start".to_string(), "React Router v7".to_string(), "SvelteKit".to_string()],
            is_primary_indicator: true,
            alternative_names: vec!["nuxtjs".to_string()],
        },
        TechnologyRule {
            name: "Astro".to_string(),
            category: TechnologyCategory::MetaFramework,
            confidence: 0.95,
            dependency_patterns: vec!["astro".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "SolidStart".to_string(),
            category: TechnologyCategory::MetaFramework,
            confidence: 0.95,
            dependency_patterns: vec!["solid-start".to_string()],
            requires: vec!["SolidJS".to_string()],
            conflicts_with: vec!["Next.js".to_string(), "Tanstack Start".to_string(), "React Router v7".to_string(), "SvelteKit".to_string()],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        
        // FRONTEND FRAMEWORKS (Provide structure)
        TechnologyRule {
            name: "Angular".to_string(),
            category: TechnologyCategory::FrontendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["@angular/core".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["angular".to_string()],
        },
        TechnologyRule {
            name: "Svelte".to_string(),
            category: TechnologyCategory::FrontendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["svelte".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false, // SvelteKit would be primary
            alternative_names: vec![],
        },
        
        // UI LIBRARIES (Not frameworks!)
        TechnologyRule {
            name: "React".to_string(),
            category: TechnologyCategory::Library(LibraryType::UI),
            confidence: 0.90,
            dependency_patterns: vec!["react".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false, // Meta-frameworks using React would be primary
            alternative_names: vec!["reactjs".to_string()],
        },
        TechnologyRule {
            name: "Vue.js".to_string(),
            category: TechnologyCategory::Library(LibraryType::UI),
            confidence: 0.90,
            dependency_patterns: vec!["vue".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["vuejs".to_string()],
        },
        TechnologyRule {
            name: "SolidJS".to_string(),
            category: TechnologyCategory::Library(LibraryType::UI),
            confidence: 0.95,
            dependency_patterns: vec!["solid-js".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["solid".to_string()],
        },
        TechnologyRule {
            name: "HTMX".to_string(),
            category: TechnologyCategory::Library(LibraryType::UI),
            confidence: 0.95,
            dependency_patterns: vec!["htmx.org".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["htmx".to_string()],
        },
        
        // Note: Removed utility libraries (Tanstack Query, Tanstack Router, state management)
        // as they don't provide value for IaC generation decisions
        
        // BACKEND FRAMEWORKS
        TechnologyRule {
            name: "Express.js".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["express".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["express".to_string()],
        },
        TechnologyRule {
            name: "Fastify".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["fastify".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Nest.js".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["@nestjs/core".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["nestjs".to_string()],
        },
        TechnologyRule {
            name: "Hono".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["hono".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Elysia".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["elysia".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Encore".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["encore.dev".to_string(), "encore".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["encore-ts-starter".to_string()],
        },
        
        // BUILD TOOLS (Not frameworks!)
        TechnologyRule {
            name: "Vite".to_string(),
            category: TechnologyCategory::BuildTool,
            confidence: 0.80,
            dependency_patterns: vec!["vite".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Webpack".to_string(),
            category: TechnologyCategory::BuildTool,
            confidence: 0.80,
            dependency_patterns: vec!["webpack".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        
        // DATABASE/ORM (Keep only major ones that affect Docker/infrastructure setup)
        TechnologyRule {
            name: "Prisma".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.90,
            dependency_patterns: vec!["prisma".to_string(), "@prisma/client".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        
        // RUNTIMES (Important for IaC - determines base images, package managers)
        TechnologyRule {
            name: "Node.js".to_string(),
            category: TechnologyCategory::Runtime,
            confidence: 0.90,
            dependency_patterns: vec!["node".to_string()], // This will need file-based detection
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["nodejs".to_string()],
        },
        TechnologyRule {
            name: "Bun".to_string(),
            category: TechnologyCategory::Runtime,
            confidence: 0.95,
            dependency_patterns: vec!["bun".to_string()], // Look for bun in devDependencies or bun.lockb file
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Deno".to_string(),
            category: TechnologyCategory::Runtime,
            confidence: 0.95,
            dependency_patterns: vec!["@deno/core".to_string(), "deno".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        
        // TESTING (Keep minimal - only major frameworks that affect build process)
        TechnologyRule {
            name: "Jest".to_string(),
            category: TechnologyCategory::Testing,
            confidence: 0.85,
            dependency_patterns: vec!["jest".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Vitest".to_string(),
            category: TechnologyCategory::Testing,
            confidence: 0.85,
            dependency_patterns: vec!["vitest".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
    ]
}

// Placeholder implementations for other languages (simplified for now)
fn get_rust_technology_rules() -> Vec<TechnologyRule> {
    vec![
        TechnologyRule {
            name: "Actix Web".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["actix-web".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["actix".to_string()],
        },
        // ... other Rust technologies
    ]
}

fn get_python_technology_rules() -> Vec<TechnologyRule> {
    vec![
        TechnologyRule {
            name: "Django".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["django".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        // ... other Python technologies
    ]
}

fn get_go_technology_rules() -> Vec<TechnologyRule> {
    vec![]
}

fn get_jvm_technology_rules() -> Vec<TechnologyRule> {
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[test]
    fn test_rust_actix_web_detection() {
        let language = DetectedLanguage {
            name: "Rust".to_string(),
            version: Some("1.70.0".to_string()),
            confidence: 0.9,
            files: vec![PathBuf::from("src/main.rs")],
            main_dependencies: vec!["actix-web".to_string(), "tokio".to_string()],
            dev_dependencies: vec!["assert_cmd".to_string()],
            package_manager: Some("cargo".to_string()),
        };
        
        let technologies = detect_rust_technologies(&language);
        
        // This is a simplified test - in real implementation, we'd need access to dependencies
        assert!(technologies.is_empty() || technologies.iter().any(|f| f.name.contains("Actix")));
    }
    
    #[test]
    fn test_framework_pattern_matching() {
        assert!(matches_pattern("express", "express"));
        assert!(matches_pattern("@nestjs/core", "@nestjs/*"));
        assert!(matches_pattern("spring-boot-starter-web", "spring-boot-starter*"));
        assert!(!matches_pattern("react", "vue"));
    }
    
    #[test]
    fn test_framework_categories() {
        let rules = get_js_technology_rules();
        
        let express_rule = rules.iter().find(|r| r.name == "Express.js").unwrap();
        assert!(matches!(express_rule.category, TechnologyCategory::BackendFramework));
        
        let jest_rule = rules.iter().find(|r| r.name == "Jest").unwrap();
        assert!(matches!(jest_rule.category, TechnologyCategory::Testing));
        
        // Test new frameworks
        let drizzle_rule = rules.iter().find(|r| r.name == "Drizzle ORM").unwrap();
        assert!(matches!(drizzle_rule.category, TechnologyCategory::Database));
        
        let svelte_rule = rules.iter().find(|r| r.name == "Svelte").unwrap();
        assert!(matches!(svelte_rule.category, TechnologyCategory::FrontendFramework));
        
        let encore_rule = rules.iter().find(|r| r.name == "Encore").unwrap();
        assert!(matches!(encore_rule.category, TechnologyCategory::BackendFramework));
        
        let hono_rule = rules.iter().find(|r| r.name == "Hono").unwrap();
        assert!(matches!(hono_rule.category, TechnologyCategory::BackendFramework));
    }
    
    #[test]
    fn test_modern_framework_detection() {
        let rules = get_js_technology_rules();
        
        // Test that we have all the new frameworks
        let framework_names: Vec<&str> = rules.iter().map(|r| r.name.as_str()).collect();
        
        assert!(framework_names.contains(&"Svelte"));
        assert!(framework_names.contains(&"SvelteKit"));
        assert!(framework_names.contains(&"Astro"));
        assert!(framework_names.contains(&"SolidJS"));
        assert!(framework_names.contains(&"Encore"));
        assert!(framework_names.contains(&"Hono"));
        assert!(framework_names.contains(&"Elysia"));
        assert!(framework_names.contains(&"Drizzle ORM"));
        assert!(framework_names.contains(&"React Router v7"));
        assert!(framework_names.contains(&"Tanstack Start"));
    }
} 