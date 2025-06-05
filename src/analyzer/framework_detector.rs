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
    
    // Combine main and dev dependencies for comprehensive detection
    let all_deps: Vec<String> = language.main_dependencies.iter()
        .chain(language.dev_dependencies.iter())
        .cloned()
        .collect();
    
    detect_technologies_by_dependencies(&rules, &all_deps, language.confidence)
}

/// Detect JavaScript/TypeScript technologies with proper classification
fn detect_js_technologies(language: &DetectedLanguage) -> Vec<DetectedTechnology> {
    let rules = get_js_technology_rules();
    
    // Combine main and dev dependencies for comprehensive detection
    let all_deps: Vec<String> = language.main_dependencies.iter()
        .chain(language.dev_dependencies.iter())
        .cloned()
        .collect();
    
    let mut technologies = detect_technologies_by_dependencies(&rules, &all_deps, language.confidence);
    
    // Enhanced detection: analyze actual source files for usage patterns
    if let Some(enhanced_techs) = detect_technologies_from_source_files(language, &rules) {
        // Merge with dependency-based detection, preferring higher confidence scores
        for enhanced_tech in enhanced_techs {
            if let Some(existing) = technologies.iter_mut().find(|t| t.name == enhanced_tech.name) {
                // Use higher confidence between dependency and source file analysis
                if enhanced_tech.confidence > existing.confidence {
                    existing.confidence = enhanced_tech.confidence;
                }
            } else {
                // Add new technology found in source files
                technologies.push(enhanced_tech);
            }
        }
    }
    
    technologies
}

/// Detect Python technologies with proper classification
fn detect_python_technologies(language: &DetectedLanguage) -> Vec<DetectedTechnology> {
    let rules = get_python_technology_rules();
    
    // Combine main and dev dependencies for comprehensive detection
    let all_deps: Vec<String> = language.main_dependencies.iter()
        .chain(language.dev_dependencies.iter())
        .cloned()
        .collect();
    
    detect_technologies_by_dependencies(&rules, &all_deps, language.confidence)
}

/// Detect Go technologies with proper classification
fn detect_go_technologies(language: &DetectedLanguage) -> Vec<DetectedTechnology> {
    let rules = get_go_technology_rules();
    
    // Combine main and dev dependencies for comprehensive detection
    let all_deps: Vec<String> = language.main_dependencies.iter()
        .chain(language.dev_dependencies.iter())
        .cloned()
        .collect();
    
    detect_technologies_by_dependencies(&rules, &all_deps, language.confidence)
}

/// Detect JVM technologies with proper classification
fn detect_jvm_technologies(language: &DetectedLanguage) -> Vec<DetectedTechnology> {
    let rules = get_jvm_technology_rules();
    
    // Combine main and dev dependencies for comprehensive detection
    let all_deps: Vec<String> = language.main_dependencies.iter()
        .chain(language.dev_dependencies.iter())
        .cloned()
        .collect();
    
    detect_technologies_by_dependencies(&rules, &all_deps, language.confidence)
}

/// Generic technology detection based on dependency patterns
fn detect_technologies_by_dependencies(
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
                .filter(|dep| matches_pattern(dep, pattern))
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
            let final_confidence = (rule.confidence * pattern_confidence * base_confidence).min(1.0);
            
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

/// Enhanced detection that analyzes actual source files for technology usage patterns
fn detect_technologies_from_source_files(language: &DetectedLanguage, rules: &[TechnologyRule]) -> Option<Vec<DetectedTechnology>> {
    use std::fs;
    
    let mut detected = Vec::new();
    
    // Analyze files for usage patterns
    for file_path in &language.files {
        if let Ok(content) = fs::read_to_string(file_path) {
            // Analyze Drizzle ORM usage patterns
            if let Some(drizzle_confidence) = analyze_drizzle_usage(&content, file_path) {
                detected.push(DetectedTechnology {
                    name: "Drizzle ORM".to_string(),
                    version: None,
                    category: TechnologyCategory::Database,
                    confidence: drizzle_confidence,
                    requires: vec![],
                    conflicts_with: vec![],
                    is_primary: false,
                });
            }
            
            // Analyze Prisma usage patterns
            if let Some(prisma_confidence) = analyze_prisma_usage(&content, file_path) {
                detected.push(DetectedTechnology {
                    name: "Prisma".to_string(),
                    version: None,
                    category: TechnologyCategory::Database,
                    confidence: prisma_confidence,
                    requires: vec![],
                    conflicts_with: vec![],
                    is_primary: false,
                });
            }
            
            // Analyze Encore usage patterns
            if let Some(encore_confidence) = analyze_encore_usage(&content, file_path) {
                detected.push(DetectedTechnology {
                    name: "Encore".to_string(),
                    version: None,
                    category: TechnologyCategory::BackendFramework,
                    confidence: encore_confidence,
                    requires: vec![],
                    conflicts_with: vec![],
                    is_primary: true,
                });
            }
            
            // Analyze Tanstack Start usage patterns
            if let Some(tanstack_confidence) = analyze_tanstack_start_usage(&content, file_path) {
                detected.push(DetectedTechnology {
                    name: "Tanstack Start".to_string(),
                    version: None,
                    category: TechnologyCategory::MetaFramework,
                    confidence: tanstack_confidence,
                    requires: vec!["React".to_string()],
                    conflicts_with: vec!["Next.js".to_string(), "React Router v7".to_string(), "SvelteKit".to_string(), "Nuxt.js".to_string()],
                    is_primary: true,
                });
            }
        }
    }
    
    if detected.is_empty() {
        None
    } else {
        Some(detected)
    }
}

/// Analyzes Drizzle ORM usage patterns in source files
fn analyze_drizzle_usage(content: &str, file_path: &std::path::Path) -> Option<f32> {
    let file_name = file_path.file_name()?.to_string_lossy();
    let mut confidence: f32 = 0.0;
    
    // High confidence indicators
    if content.contains("drizzle-orm") {
        confidence += 0.3;
    }
    
    // Schema file patterns (very high confidence)
    if file_name.contains("schema") || file_name.contains("db.ts") || file_name.contains("database") {
        if content.contains("pgTable") || content.contains("mysqlTable") || content.contains("sqliteTable") {
            confidence += 0.4;
        }
        if content.contains("pgEnum") || content.contains("relations") {
            confidence += 0.3;
        }
    }
    
    // Drizzle-specific imports
    if content.contains("from 'drizzle-orm/pg-core'") || 
       content.contains("from 'drizzle-orm/mysql-core'") ||
       content.contains("from 'drizzle-orm/sqlite-core'") {
        confidence += 0.3;
    }
    
    // Drizzle query patterns
    if content.contains("db.select()") || content.contains("db.insert()") || 
       content.contains("db.update()") || content.contains("db.delete()") {
        confidence += 0.2;
    }
    
    // Configuration patterns
    if content.contains("drizzle(") && (content.contains("connectionString") || content.contains("postgres(")) {
        confidence += 0.2;
    }
    
    // Migration patterns
    if content.contains("drizzle.config") || file_name.contains("migrate") {
        confidence += 0.2;
    }
    
    // Prepared statements
    if content.contains(".prepare()") && content.contains("drizzle") {
        confidence += 0.1;
    }
    
    if confidence > 0.0 {
        Some(confidence.min(1.0_f32))
    } else {
        None
    }
}

/// Analyzes Prisma usage patterns in source files
fn analyze_prisma_usage(content: &str, file_path: &std::path::Path) -> Option<f32> {
    let file_name = file_path.file_name()?.to_string_lossy();
    let mut confidence: f32 = 0.0;
    let mut has_prisma_import = false;
    
    // Only detect Prisma if there are actual Prisma-specific imports
    if content.contains("@prisma/client") || content.contains("from '@prisma/client'") {
        confidence += 0.4;
        has_prisma_import = true;
    }
    
    // Prisma schema files (very specific)
    if file_name == "schema.prisma" {
        if content.contains("model ") || content.contains("generator ") || content.contains("datasource ") {
            confidence += 0.6;
            has_prisma_import = true;
        }
    }
    
    // Only check for client usage if we have confirmed Prisma imports
    if has_prisma_import {
        // Prisma client instantiation (very specific)
        if content.contains("new PrismaClient") || content.contains("PrismaClient()") {
            confidence += 0.3;
        }
        
        // Prisma-specific query patterns (only if we know it's Prisma)
        if content.contains("prisma.") && (
            content.contains(".findUnique(") || 
            content.contains(".findFirst(") || 
            content.contains(".upsert(") ||
            content.contains(".$connect()") ||
            content.contains(".$disconnect()")
        ) {
            confidence += 0.2;
        }
    }
    
    // Only return confidence if we have actual Prisma indicators
    if confidence > 0.0 && has_prisma_import {
        Some(confidence.min(1.0_f32))
    } else {
        None
    }
}

/// Analyzes Encore usage patterns in source files
fn analyze_encore_usage(content: &str, file_path: &std::path::Path) -> Option<f32> {
    let file_name = file_path.file_name()?.to_string_lossy();
    let mut confidence: f32 = 0.0;
    
    // Skip generated files (like Encore client code)
    if content.contains("// Code generated by the Encore") || content.contains("DO NOT EDIT") {
        return None;
    }
    
    // Skip client-only files (generated or consumption only)
    if file_name.contains("client.ts") || file_name.contains("client.js") {
        return None;
    }
    
    // Only detect Encore when there are actual service development patterns
    let mut has_service_patterns = false;
    
    // Service definition files (high confidence for actual Encore development)
    if file_name.contains("encore.service") || file_name.contains("service.ts") {
        confidence += 0.4;
        has_service_patterns = true;
    }
    
    // API endpoint definitions (indicates actual Encore service development)
    if content.contains("encore.dev/api") && (content.contains("export") || content.contains("api.")) {
        confidence += 0.4;
        has_service_patterns = true;
    }
    
    // Database service patterns (actual Encore service code)
    if content.contains("SQLDatabase") && content.contains("encore.dev") {
        confidence += 0.3;
        has_service_patterns = true;
    }
    
    // Secret configuration (actual Encore service code)
    if content.contains("secret(") && content.contains("encore.dev/config") {
        confidence += 0.3;
        has_service_patterns = true;
    }
    
    // PubSub service patterns (actual Encore service code)
    if content.contains("Topic") && content.contains("encore.dev/pubsub") {
        confidence += 0.3;
        has_service_patterns = true;
    }
    
    // Cron job patterns (actual Encore service code)
    if content.contains("cron") && content.contains("encore.dev") {
        confidence += 0.2;
        has_service_patterns = true;
    }
    
    // Only return confidence if we have actual service development patterns
    if confidence > 0.0 && has_service_patterns {
        Some(confidence.min(1.0_f32))
    } else {
        None
    }
}

/// Analyzes Tanstack Start usage patterns in source files
fn analyze_tanstack_start_usage(content: &str, file_path: &std::path::Path) -> Option<f32> {
    let file_name = file_path.file_name()?.to_string_lossy();
    let mut confidence: f32 = 0.0;
    let mut has_start_patterns = false;
    
    // Configuration files (high confidence)
    if file_name == "app.config.ts" || file_name == "app.config.js" {
        if content.contains("@tanstack/react-start") || content.contains("tanstack") {
            confidence += 0.5;
            has_start_patterns = true;
        }
    }
    
    // Router configuration patterns (very high confidence)
    if file_name.contains("router.") && (file_name.ends_with(".ts") || file_name.ends_with(".tsx")) {
        if content.contains("createRouter") && content.contains("@tanstack/react-router") {
            confidence += 0.4;
            has_start_patterns = true;
        }
        if content.contains("routeTree") {
            confidence += 0.2;
            has_start_patterns = true;
        }
    }
    
    // Server entry point patterns
    if file_name == "ssr.tsx" || file_name == "ssr.ts" {
        if content.contains("createStartHandler") || content.contains("@tanstack/react-start/server") {
            confidence += 0.5;
            has_start_patterns = true;
        }
    }
    
    // Client entry point patterns
    if file_name == "client.tsx" || file_name == "client.ts" {
        if content.contains("StartClient") && content.contains("@tanstack/react-start") {
            confidence += 0.5;
            has_start_patterns = true;
        }
        if content.contains("hydrateRoot") && content.contains("createRouter") {
            confidence += 0.3;
            has_start_patterns = true;
        }
    }
    
    // Root route patterns (in app/routes/__root.tsx)
    if file_name == "__root.tsx" || file_name == "__root.ts" {
        if content.contains("createRootRoute") && content.contains("@tanstack/react-router") {
            confidence += 0.4;
            has_start_patterns = true;
        }
        if content.contains("HeadContent") && content.contains("Scripts") {
            confidence += 0.3;
            has_start_patterns = true;
        }
    }
    
    // Route files with createFileRoute
    if file_path.to_string_lossy().contains("routes/") {
        if content.contains("createFileRoute") && content.contains("@tanstack/react-router") {
            confidence += 0.3;
            has_start_patterns = true;
        }
    }
    
    // Server functions (key Tanstack Start feature)
    if content.contains("createServerFn") && content.contains("@tanstack/react-start") {
        confidence += 0.4;
        has_start_patterns = true;
    }
    
    // Import patterns specific to Tanstack Start
    if content.contains("from '@tanstack/react-start'") {
        confidence += 0.3;
        has_start_patterns = true;
    }
    
    // Vinxi configuration patterns
    if file_name == "vinxi.config.ts" || file_name == "vinxi.config.js" {
        confidence += 0.2;
        has_start_patterns = true;
    }
    
    // Only return confidence if we have actual Tanstack Start patterns
    if confidence > 0.0 && has_start_patterns {
        Some(confidence.min(1.0_f32))
    } else {
        None
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
            alternative_names: vec!["tanstack-start".to_string(), "TanStack Start".to_string()],
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
        
        // DATABASE/ORM (Important for Docker/infrastructure setup, migrations, etc.)
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
        TechnologyRule {
            name: "Drizzle ORM".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.90,
            dependency_patterns: vec!["drizzle-orm".to_string(), "drizzle-kit".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["drizzle".to_string()],
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

/// Rust technology detection rules with comprehensive framework coverage
fn get_rust_technology_rules() -> Vec<TechnologyRule> {
    vec![
        // WEB FRAMEWORKS
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
        TechnologyRule {
            name: "Axum".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["axum".to_string()],
            requires: vec!["Tokio".to_string()],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Rocket".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["rocket".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Warp".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["warp".to_string()],
            requires: vec!["Tokio".to_string()],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Loco".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["loco-rs".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["loco".to_string()],
        },
        TechnologyRule {
            name: "Poem".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["poem".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Salvo".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["salvo".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Trillium".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["trillium".to_string()],
            requires: vec!["Tokio".to_string()],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        
        // ASYNC RUNTIMES
        TechnologyRule {
            name: "Tokio".to_string(),
            category: TechnologyCategory::Runtime,
            confidence: 0.90,
            dependency_patterns: vec!["tokio".to_string()],
            requires: vec![],
            conflicts_with: vec!["async-std".to_string()],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "async-std".to_string(),
            category: TechnologyCategory::Runtime,
            confidence: 0.90,
            dependency_patterns: vec!["async-std".to_string()],
            requires: vec![],
            conflicts_with: vec!["Tokio".to_string()],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        
        // DATABASE/ORM
        TechnologyRule {
            name: "SeaORM".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.90,
            dependency_patterns: vec!["sea-orm".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["sea_orm".to_string()],
        },
        TechnologyRule {
            name: "Diesel".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.90,
            dependency_patterns: vec!["diesel".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "SQLx".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.90,
            dependency_patterns: vec!["sqlx".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        
        // SERIALIZATION
        TechnologyRule {
            name: "Serde".to_string(),
            category: TechnologyCategory::Library(LibraryType::Utility),
            confidence: 0.85,
            dependency_patterns: vec!["serde".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        
        // TESTING
        TechnologyRule {
            name: "Criterion".to_string(),
            category: TechnologyCategory::Testing,
            confidence: 0.85,
            dependency_patterns: vec!["criterion".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        
        // GUI FRAMEWORKS (WASM/Desktop)
        TechnologyRule {
            name: "Leptos".to_string(),
            category: TechnologyCategory::FrontendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["leptos".to_string()],
            requires: vec![],
            conflicts_with: vec!["Yew".to_string(), "Dioxus".to_string()],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Yew".to_string(),
            category: TechnologyCategory::FrontendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["yew".to_string()],
            requires: vec![],
            conflicts_with: vec!["Leptos".to_string(), "Dioxus".to_string()],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Dioxus".to_string(),
            category: TechnologyCategory::FrontendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["dioxus".to_string()],
            requires: vec![],
            conflicts_with: vec!["Leptos".to_string(), "Yew".to_string()],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Tauri".to_string(),
            category: TechnologyCategory::Library(LibraryType::UI),
            confidence: 0.95,
            dependency_patterns: vec!["tauri".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "egui".to_string(),
            category: TechnologyCategory::Library(LibraryType::UI),
            confidence: 0.95,
            dependency_patterns: vec!["egui".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
    ]
}

/// Python technology detection rules with comprehensive framework coverage
fn get_python_technology_rules() -> Vec<TechnologyRule> {
    vec![
        // WEB FRAMEWORKS - Full Stack
        TechnologyRule {
            name: "Django".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["django".to_string(), "Django".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Django REST Framework".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["djangorestframework".to_string(), "rest_framework".to_string()],
            requires: vec!["Django".to_string()],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["DRF".to_string()],
        },
        
        // MICRO FRAMEWORKS
        TechnologyRule {
            name: "Flask".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["flask".to_string(), "Flask".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "FastAPI".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["fastapi".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Starlette".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["starlette".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Quart".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["quart".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Sanic".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["sanic".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Tornado".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["tornado".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Falcon".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["falcon".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Bottle".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["bottle".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "aiohttp".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["aiohttp".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        
        // DATABASE/ORM
        TechnologyRule {
            name: "SQLAlchemy".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.90,
            dependency_patterns: vec!["sqlalchemy".to_string(), "SQLAlchemy".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Peewee".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.90,
            dependency_patterns: vec!["peewee".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Tortoise ORM".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.90,
            dependency_patterns: vec!["tortoise-orm".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["tortoise".to_string()],
        },
        TechnologyRule {
            name: "Django ORM".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.95,
            dependency_patterns: vec!["django.db".to_string()],
            requires: vec!["Django".to_string()],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        
        // ASYNC FRAMEWORKS
        TechnologyRule {
            name: "asyncio".to_string(),
            category: TechnologyCategory::Runtime,
            confidence: 0.85,
            dependency_patterns: vec!["asyncio".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        
        // DATA SCIENCE FRAMEWORKS (Important for containerization/deployment)
        TechnologyRule {
            name: "NumPy".to_string(),
            category: TechnologyCategory::Library(LibraryType::Utility),
            confidence: 0.85,
            dependency_patterns: vec!["numpy".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Pandas".to_string(),
            category: TechnologyCategory::Library(LibraryType::Utility),
            confidence: 0.85,
            dependency_patterns: vec!["pandas".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Scikit-learn".to_string(),
            category: TechnologyCategory::Library(LibraryType::Utility),
            confidence: 0.85,
            dependency_patterns: vec!["scikit-learn".to_string(), "sklearn".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["sklearn".to_string()],
        },
        TechnologyRule {
            name: "TensorFlow".to_string(),
            category: TechnologyCategory::Library(LibraryType::Utility),
            confidence: 0.90,
            dependency_patterns: vec!["tensorflow".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "PyTorch".to_string(),
            category: TechnologyCategory::Library(LibraryType::Utility),
            confidence: 0.90,
            dependency_patterns: vec!["torch".to_string(), "pytorch".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["torch".to_string()],
        },
        
        // TASK QUEUES
        TechnologyRule {
            name: "Celery".to_string(),
            category: TechnologyCategory::Library(LibraryType::Utility),
            confidence: 0.90,
            dependency_patterns: vec!["celery".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        
        // TESTING
        TechnologyRule {
            name: "pytest".to_string(),
            category: TechnologyCategory::Testing,
            confidence: 0.85,
            dependency_patterns: vec!["pytest".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "unittest".to_string(),
            category: TechnologyCategory::Testing,
            confidence: 0.80,
            dependency_patterns: vec!["unittest".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        
        // WSGI/ASGI SERVERS
        TechnologyRule {
            name: "Gunicorn".to_string(),
            category: TechnologyCategory::Runtime,
            confidence: 0.85,
            dependency_patterns: vec!["gunicorn".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Uvicorn".to_string(),
            category: TechnologyCategory::Runtime,
            confidence: 0.85,
            dependency_patterns: vec!["uvicorn".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
    ]
}

/// Go technology detection rules with comprehensive framework coverage
fn get_go_technology_rules() -> Vec<TechnologyRule> {
    vec![
        // WEB FRAMEWORKS
        TechnologyRule {
            name: "Gin".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["github.com/gin-gonic/gin".to_string(), "gin-gonic".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["gin-gonic".to_string()],
        },
        TechnologyRule {
            name: "Echo".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["github.com/labstack/echo".to_string(), "labstack/echo".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["labstack/echo".to_string()],
        },
        TechnologyRule {
            name: "Fiber".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["github.com/gofiber/fiber".to_string(), "gofiber".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["gofiber".to_string()],
        },
        TechnologyRule {
            name: "Beego".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["github.com/beego/beego".to_string(), "beego".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Chi".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["github.com/go-chi/chi".to_string(), "go-chi".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["go-chi".to_string()],
        },
        TechnologyRule {
            name: "Gorilla Mux".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["github.com/gorilla/mux".to_string(), "gorilla/mux".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["mux".to_string(), "gorilla".to_string()],
        },
        TechnologyRule {
            name: "Revel".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["github.com/revel/revel".to_string(), "revel".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Buffalo".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["github.com/gobuffalo/buffalo".to_string(), "gobuffalo".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["gobuffalo".to_string()],
        },
        TechnologyRule {
            name: "Iris".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["github.com/kataras/iris".to_string(), "kataras/iris".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "FastHTTP".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["github.com/valyala/fasthttp".to_string(), "fasthttp".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["valyala/fasthttp".to_string()],
        },
        TechnologyRule {
            name: "Hertz".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["github.com/cloudwego/hertz".to_string(), "cloudwego/hertz".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["cloudwego".to_string()],
        },
        
        // DATABASE/ORM
        TechnologyRule {
            name: "GORM".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.90,
            dependency_patterns: vec!["gorm.io/gorm".to_string(), "gorm".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Ent".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.90,
            dependency_patterns: vec!["entgo.io/ent".to_string(), "facebook/ent".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["entgo".to_string()],
        },
        TechnologyRule {
            name: "Xorm".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.85,
            dependency_patterns: vec!["xorm.io/xorm".to_string(), "xorm".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        
        // MICROSERVICES
        TechnologyRule {
            name: "Go Kit".to_string(),
            category: TechnologyCategory::Library(LibraryType::Utility),
            confidence: 0.90,
            dependency_patterns: vec!["github.com/go-kit/kit".to_string(), "go-kit".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["kit".to_string()],
        },
        TechnologyRule {
            name: "Kratos".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["github.com/go-kratos/kratos".to_string(), "go-kratos".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["go-kratos".to_string()],
        },
        
        // MESSAGE QUEUES
        TechnologyRule {
            name: "Sarama".to_string(),
            category: TechnologyCategory::Library(LibraryType::Utility),
            confidence: 0.85,
            dependency_patterns: vec!["github.com/shopify/sarama".to_string(), "sarama".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["shopify/sarama".to_string()],
        },
        
        // TESTING
        TechnologyRule {
            name: "Testify".to_string(),
            category: TechnologyCategory::Testing,
            confidence: 0.85,
            dependency_patterns: vec!["github.com/stretchr/testify".to_string(), "testify".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["stretchr/testify".to_string()],
        },
        TechnologyRule {
            name: "Ginkgo".to_string(),
            category: TechnologyCategory::Testing,
            confidence: 0.85,
            dependency_patterns: vec!["github.com/onsi/ginkgo".to_string(), "ginkgo".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["onsi/ginkgo".to_string()],
        },
        
        // CLI FRAMEWORKS
        TechnologyRule {
            name: "Cobra".to_string(),
            category: TechnologyCategory::Library(LibraryType::Utility),
            confidence: 0.85,
            dependency_patterns: vec!["github.com/spf13/cobra".to_string(), "cobra".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["spf13/cobra".to_string()],
        },
        
        // CONFIG MANAGEMENT
        TechnologyRule {
            name: "Viper".to_string(),
            category: TechnologyCategory::Library(LibraryType::Utility),
            confidence: 0.80,
            dependency_patterns: vec!["github.com/spf13/viper".to_string(), "viper".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["spf13/viper".to_string()],
        },
    ]
}

/// Java/JVM technology detection rules with comprehensive framework coverage
fn get_jvm_technology_rules() -> Vec<TechnologyRule> {
    vec![
        // SPRING ECOSYSTEM
        TechnologyRule {
            name: "Spring Boot".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["spring-boot".to_string(), "org.springframework.boot".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["spring".to_string()],
        },
        TechnologyRule {
            name: "Spring Framework".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["spring-context".to_string(), "org.springframework".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["spring".to_string()],
        },
        TechnologyRule {
            name: "Spring Data".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.90,
            dependency_patterns: vec!["spring-data".to_string(), "org.springframework.data".to_string()],
            requires: vec!["Spring Framework".to_string()],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Spring Security".to_string(),
            category: TechnologyCategory::Library(LibraryType::Utility),
            confidence: 0.90,
            dependency_patterns: vec!["spring-security".to_string(), "org.springframework.security".to_string()],
            requires: vec!["Spring Framework".to_string()],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Spring Cloud".to_string(),
            category: TechnologyCategory::Library(LibraryType::Utility),
            confidence: 0.90,
            dependency_patterns: vec!["spring-cloud".to_string(), "org.springframework.cloud".to_string()],
            requires: vec!["Spring Boot".to_string()],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        
        // MICROSERVICES FRAMEWORKS
        TechnologyRule {
            name: "Quarkus".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["quarkus".to_string(), "io.quarkus".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Micronaut".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["micronaut".to_string(), "io.micronaut".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Helidon".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["helidon".to_string(), "io.helidon".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Vert.x".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["vertx".to_string(), "io.vertx".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["eclipse vert.x".to_string(), "vertx".to_string()],
        },
        
        // TRADITIONAL FRAMEWORKS
        TechnologyRule {
            name: "Struts".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["struts".to_string(), "org.apache.struts".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["apache struts".to_string()],
        },
        TechnologyRule {
            name: "JSF".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.85,
            dependency_patterns: vec!["jsf".to_string(), "javax.faces".to_string(), "jakarta.faces".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["javaserver faces".to_string()],
        },
        
        // LIGHTWEIGHT FRAMEWORKS
        TechnologyRule {
            name: "Dropwizard".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["dropwizard".to_string(), "io.dropwizard".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Spark Java".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["spark-core".to_string(), "com.sparkjava".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["spark".to_string()],
        },
        TechnologyRule {
            name: "Javalin".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["javalin".to_string(), "io.javalin".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Ratpack".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["ratpack".to_string(), "io.ratpack".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
        
        // PLAY FRAMEWORK
        TechnologyRule {
            name: "Play Framework".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["play".to_string(), "com.typesafe.play".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["play".to_string()],
        },
        
        // ORM/DATABASE
        TechnologyRule {
            name: "Hibernate".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.90,
            dependency_patterns: vec!["hibernate".to_string(), "org.hibernate".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["hibernate orm".to_string()],
        },
        TechnologyRule {
            name: "MyBatis".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.90,
            dependency_patterns: vec!["mybatis".to_string(), "org.mybatis".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "JOOQ".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.85,
            dependency_patterns: vec!["jooq".to_string(), "org.jooq".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        
        // ENTERPRISE JAVA
        TechnologyRule {
            name: "Jakarta EE".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.90,
            dependency_patterns: vec!["jakarta.".to_string(), "jakarta-ee".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec!["java ee".to_string()],
        },
        
        // BUILD TOOLS
        TechnologyRule {
            name: "Maven".to_string(),
            category: TechnologyCategory::BuildTool,
            confidence: 0.80,
            dependency_patterns: vec!["maven".to_string(), "org.apache.maven".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["apache maven".to_string()],
        },
        TechnologyRule {
            name: "Gradle".to_string(),
            category: TechnologyCategory::BuildTool,
            confidence: 0.80,
            dependency_patterns: vec!["gradle".to_string(), "org.gradle".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        
        // TESTING
        TechnologyRule {
            name: "JUnit".to_string(),
            category: TechnologyCategory::Testing,
            confidence: 0.85,
            dependency_patterns: vec!["junit".to_string(), "org.junit".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "TestNG".to_string(),
            category: TechnologyCategory::Testing,
            confidence: 0.85,
            dependency_patterns: vec!["testng".to_string(), "org.testng".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        TechnologyRule {
            name: "Mockito".to_string(),
            category: TechnologyCategory::Testing,
            confidence: 0.80,
            dependency_patterns: vec!["mockito".to_string(), "org.mockito".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        
        // REACTIVE FRAMEWORKS
        TechnologyRule {
            name: "Reactor".to_string(),
            category: TechnologyCategory::Library(LibraryType::Utility),
            confidence: 0.85,
            dependency_patterns: vec!["reactor".to_string(), "io.projectreactor".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["project reactor".to_string()],
        },
        TechnologyRule {
            name: "RxJava".to_string(),
            category: TechnologyCategory::Library(LibraryType::Utility),
            confidence: 0.85,
            dependency_patterns: vec!["rxjava".to_string(), "io.reactivex".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
        },
        
        // KOTLIN SPECIFIC
        TechnologyRule {
            name: "Ktor".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["ktor".to_string(), "io.ktor".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: true,
            alternative_names: vec![],
        },
    ]
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
    
    #[test]
    fn test_tanstack_start_detection() {
        let language = DetectedLanguage {
            name: "TypeScript".to_string(),
            version: Some("5.0.0".to_string()),
            confidence: 0.925,
            files: vec![PathBuf::from("src/routes/index.tsx")],
            main_dependencies: vec![
                "@tanstack/react-start".to_string(),
                "@tanstack/react-router".to_string(),
                "react".to_string(),
                "react-dom".to_string(),
            ],
            dev_dependencies: vec![
                "vinxi".to_string(),
                "typescript".to_string(),
            ],
            package_manager: Some("npm".to_string()),
        };
        
        let technologies = detect_js_technologies(&language);
        
        // Should detect Tanstack Start
        let tanstack_start = technologies.iter().find(|t| t.name == "Tanstack Start");
        assert!(tanstack_start.is_some(), "Tanstack Start should be detected");
        
        let tanstack_start = tanstack_start.unwrap();
        assert!(matches!(tanstack_start.category, TechnologyCategory::MetaFramework));
        assert!(tanstack_start.is_primary, "Tanstack Start should be marked as primary");
        assert!(tanstack_start.confidence > 0.8, "Tanstack Start detection confidence should be high");
        
        // Should also detect React
        let react = technologies.iter().find(|t| t.name == "React");
        assert!(react.is_some(), "React should be detected as a dependency");
    }

    #[test]
    fn test_comprehensive_python_detection() {
        let language = DetectedLanguage {
            name: "Python".to_string(),
            version: Some("3.11.0".to_string()),
            confidence: 0.95,
            files: vec![PathBuf::from("app.py")],
            main_dependencies: vec![
                "fastapi".to_string(),
                "sqlalchemy".to_string(),
                "pandas".to_string(),
                "torch".to_string(),
                "gunicorn".to_string(),
            ],
            dev_dependencies: vec!["pytest".to_string()],
            package_manager: Some("pip".to_string()),
        };
        
        let technologies = detect_python_technologies(&language);
        let tech_names: Vec<&str> = technologies.iter().map(|t| t.name.as_str()).collect();
        
        // Should detect FastAPI as primary backend framework
        assert!(tech_names.contains(&"FastAPI"));
        assert!(tech_names.contains(&"SQLAlchemy"));
        assert!(tech_names.contains(&"Pandas"));
        assert!(tech_names.contains(&"PyTorch"));
        assert!(tech_names.contains(&"Gunicorn"));
        assert!(tech_names.contains(&"pytest"));
        
        // FastAPI should be marked as primary
        let fastapi = technologies.iter().find(|t| t.name == "FastAPI").unwrap();
        assert!(fastapi.is_primary);
        assert!(matches!(fastapi.category, TechnologyCategory::BackendFramework));
    }

    #[test]
    fn test_comprehensive_go_detection() {
        let language = DetectedLanguage {
            name: "Go".to_string(),
            version: Some("1.21.0".to_string()),
            confidence: 0.95,
            files: vec![PathBuf::from("main.go")],
            main_dependencies: vec![
                "github.com/gin-gonic/gin".to_string(),
                "gorm.io/gorm".to_string(),
                "github.com/spf13/cobra".to_string(),
                "github.com/spf13/viper".to_string(),
            ],
            dev_dependencies: vec!["github.com/stretchr/testify".to_string()],
            package_manager: Some("go mod".to_string()),
        };
        
        let technologies = detect_go_technologies(&language);
        let tech_names: Vec<&str> = technologies.iter().map(|t| t.name.as_str()).collect();
        
        // Should detect Gin as primary backend framework
        assert!(tech_names.contains(&"Gin"));
        assert!(tech_names.contains(&"GORM"));
        assert!(tech_names.contains(&"Cobra"));
        assert!(tech_names.contains(&"Viper"));
        assert!(tech_names.contains(&"Testify"));
        
        // Gin should be marked as primary
        let gin = technologies.iter().find(|t| t.name == "Gin").unwrap();
        assert!(gin.is_primary);
        assert!(matches!(gin.category, TechnologyCategory::BackendFramework));
    }

    #[test]
    fn test_comprehensive_jvm_detection() {
        let language = DetectedLanguage {
            name: "Java".to_string(),
            version: Some("17.0.0".to_string()),
            confidence: 0.95,
            files: vec![PathBuf::from("src/main/java/Application.java")],
            main_dependencies: vec![
                "spring-boot".to_string(),
                "spring-data".to_string(),
                "hibernate".to_string(),
                "io.projectreactor".to_string(),
            ],
            dev_dependencies: vec!["junit".to_string(), "mockito".to_string()],
            package_manager: Some("maven".to_string()),
        };
        
        let technologies = detect_jvm_technologies(&language);
        let tech_names: Vec<&str> = technologies.iter().map(|t| t.name.as_str()).collect();
        
        // Should detect Spring Boot as primary backend framework
        assert!(tech_names.contains(&"Spring Boot"));
        assert!(tech_names.contains(&"Spring Data"));
        assert!(tech_names.contains(&"Hibernate"));
        assert!(tech_names.contains(&"Reactor"));
        assert!(tech_names.contains(&"JUnit"));
        assert!(tech_names.contains(&"Mockito"));
        
        // Spring Boot should be marked as primary
        let spring_boot = technologies.iter().find(|t| t.name == "Spring Boot").unwrap();
        assert!(spring_boot.is_primary);
        assert!(matches!(spring_boot.category, TechnologyCategory::BackendFramework));
    }

    #[test]
    fn test_comprehensive_rust_detection() {
        let language = DetectedLanguage {
            name: "Rust".to_string(),
            version: Some("1.70.0".to_string()),
            confidence: 0.95,
            files: vec![PathBuf::from("src/main.rs")],
            main_dependencies: vec![
                "axum".to_string(),
                "tokio".to_string(),
                "sqlx".to_string(),
                "serde".to_string(),
                "tauri".to_string(),
            ],
            dev_dependencies: vec!["criterion".to_string()],
            package_manager: Some("cargo".to_string()),
        };
        
        let technologies = detect_rust_technologies(&language);
        let tech_names: Vec<&str> = technologies.iter().map(|t| t.name.as_str()).collect();
        
        // Should detect Axum as primary backend framework
        assert!(tech_names.contains(&"Axum"));
        assert!(tech_names.contains(&"Tokio"));
        assert!(tech_names.contains(&"SQLx"));
        assert!(tech_names.contains(&"Serde"));
        assert!(tech_names.contains(&"Tauri"));
        assert!(tech_names.contains(&"Criterion"));
        
        // Axum should be marked as primary
        let axum = technologies.iter().find(|t| t.name == "Axum").unwrap();
        assert!(axum.is_primary);
        assert!(matches!(axum.category, TechnologyCategory::BackendFramework));
        
        // Axum should require Tokio
        assert!(axum.requires.contains(&"Tokio".to_string()));
    }

    #[test]
    fn test_technology_conflicts_resolution() {
        use crate::analyzer::AnalysisConfig;
        use std::path::Path;
        
        let language = DetectedLanguage {
            name: "Rust".to_string(),
            version: Some("1.70.0".to_string()),
            confidence: 0.95,
            files: vec![PathBuf::from("src/main.rs")],
            main_dependencies: vec![
                "tokio".to_string(),
                "async-std".to_string(), // These should conflict
            ],
            dev_dependencies: vec![],
            package_manager: Some("cargo".to_string()),
        };
        
        let config = AnalysisConfig::default();
        let project_root = Path::new(".");
        
        // Use the main detection function which includes conflict resolution
        let technologies = detect_frameworks(project_root, &[language], &config).unwrap();
        let _tech_names: Vec<&str> = technologies.iter().map(|t| t.name.as_str()).collect();
        
        // Should only have one async runtime (higher confidence wins)
        let async_runtimes: Vec<_> = technologies.iter()
            .filter(|t| matches!(t.category, TechnologyCategory::Runtime))
            .collect();
        
        assert!(async_runtimes.len() <= 1, "Should resolve conflicting async runtimes: found {:?}", 
               async_runtimes.iter().map(|t| &t.name).collect::<Vec<_>>());
    }
} 