use super::{LanguageFrameworkDetector, TechnologyRule, FrameworkDetectionUtils};
use crate::analyzer::{DetectedTechnology, DetectedLanguage, TechnologyCategory, LibraryType};
use crate::error::Result;
use std::fs;
use std::path::Path;

pub struct JavaScriptFrameworkDetector;

impl LanguageFrameworkDetector for JavaScriptFrameworkDetector {
    fn detect_frameworks(&self, language: &DetectedLanguage) -> Result<Vec<DetectedTechnology>> {
        let rules = get_js_technology_rules();
        
        // New: Enhanced detection using file-based approach first
        let mut technologies = detect_frameworks_from_files(language, &rules)?;
        
        // Combine main and dev dependencies for comprehensive detection
        let all_deps: Vec<String> = language.main_dependencies.iter()
            .chain(language.dev_dependencies.iter())
            .cloned()
            .collect();
        
        // Enhanced detection: analyze actual source files for usage patterns
        if let Some(enhanced_techs) = detect_technologies_from_source_files(language, &rules) {
            // Merge with file-based detection, preferring higher confidence scores
            for enhanced_tech in enhanced_techs {
                if let Some(existing) = technologies.iter_mut().find(|t| t.name == enhanced_tech.name) {
                    // Use higher confidence between file-based and source file analysis
                    if enhanced_tech.confidence > existing.confidence {
                        existing.confidence = enhanced_tech.confidence;
                    }
                } else {
                    // Add new technology found in source files
                    technologies.push(enhanced_tech);
                }
            }
        }
        
        // Fallback to dependency-based detection
        let dependency_based_techs = FrameworkDetectionUtils::detect_technologies_by_dependencies(
            &rules, &all_deps, language.confidence
        );
        
        // Merge dependency-based detections with higher confidence scores
        for dep_tech in dependency_based_techs {
            if let Some(existing) = technologies.iter_mut().find(|t| t.name == dep_tech.name) {
                // Use higher confidence between file-based and dependency-based detection
                if dep_tech.confidence > existing.confidence {
                    existing.confidence = dep_tech.confidence;
                }
            } else {
                // Add new technology found through dependencies
                technologies.push(dep_tech);
            }
        }
        
        Ok(technologies)
    }
    
    fn supported_languages(&self) -> Vec<&'static str> {
        vec!["JavaScript", "TypeScript", "JavaScript/TypeScript"]
    }
}

/// New: Enhanced detection that analyzes project files for framework indicators
fn detect_frameworks_from_files(language: &DetectedLanguage, rules: &[TechnologyRule]) -> Result<Vec<DetectedTechnology>> {
    let mut detected = Vec::new();
    
    // Check for configuration files first (highest priority)
    if let Some(config_detections) = detect_by_config_files(language, rules) {
        detected.extend(config_detections);
    }
    
    // If no config-based detections, check project structure (medium priority)
    if detected.is_empty() {
        if let Some(structure_detections) = detect_by_project_structure(language, rules) {
            detected.extend(structure_detections);
        }
    }
    
    // Check source code patterns (lower priority)
    if let Some(source_detections) = detect_by_source_patterns(language, rules) {
        // Merge with existing detections, preferring higher confidence
        for source_tech in source_detections {
            if let Some(existing_tech) = detected.iter_mut().find(|t| t.name == source_tech.name) {
                if source_tech.confidence > existing_tech.confidence {
                    existing_tech.confidence = source_tech.confidence;
                }
            } else {
                detected.push(source_tech);
            }
        }
    }
    
    Ok(detected)
}

/// New: Detect frameworks by checking for framework-specific configuration files
fn detect_by_config_files(language: &DetectedLanguage, rules: &[TechnologyRule]) -> Option<Vec<DetectedTechnology>> {
    let mut detected = Vec::new();
    
    // Check each file in the project for config files
    for file_path in &language.files {
        if let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) {
            // Check for Expo config files
            if file_name == "app.json" || file_name == "app.config.js" || file_name == "app.config.ts" {
                // For app.config files, we need to check the content to distinguish between Expo and TanStack Start
                // But for testing purposes, we'll make assumptions based on file names and dependencies
                if file_name == "app.config.js" || file_name == "app.config.ts" {
                    // Check if we have Expo dependencies
                    let has_expo_deps = language.main_dependencies.iter().any(|dep| dep == "expo" || dep == "react-native");
                    let has_tanstack_deps = language.main_dependencies.iter().any(|dep| dep.contains("tanstack") || dep.contains("vinxi"));
                    
                    if has_expo_deps && !has_tanstack_deps {
                        if let Some(expo_rule) = rules.iter().find(|r| r.name == "Expo") {
                            detected.push(DetectedTechnology {
                                name: expo_rule.name.clone(),
                                version: None,
                                category: expo_rule.category.clone(),
                                confidence: 1.0, // High confidence from config file with Expo content
                                requires: expo_rule.requires.clone(),
                                conflicts_with: expo_rule.conflicts_with.clone(),
                                is_primary: expo_rule.is_primary_indicator,
                                file_indicators: expo_rule.file_indicators.clone(),
                            });
                        }
                    } else if has_tanstack_deps && !has_expo_deps {
                        if let Some(tanstack_rule) = rules.iter().find(|r| r.name == "Tanstack Start") {
                            detected.push(DetectedTechnology {
                                name: tanstack_rule.name.clone(),
                                version: None,
                                category: tanstack_rule.category.clone(),
                                confidence: 1.0, // High confidence from config file with TanStack content
                                requires: tanstack_rule.requires.clone(),
                                conflicts_with: tanstack_rule.conflicts_with.clone(),
                                is_primary: tanstack_rule.is_primary_indicator,
                                file_indicators: tanstack_rule.file_indicators.clone(),
                            });
                        }
                    }
                    // If we can't determine, we'll skip for now
                } else {
                    // For app.json, we can assume it's Expo
                    if let Some(expo_rule) = rules.iter().find(|r| r.name == "Expo") {
                        detected.push(DetectedTechnology {
                            name: expo_rule.name.clone(),
                            version: None,
                            category: expo_rule.category.clone(),
                            confidence: 1.0, // High confidence from config file
                            requires: expo_rule.requires.clone(),
                            conflicts_with: expo_rule.conflicts_with.clone(),
                            is_primary: expo_rule.is_primary_indicator,
                            file_indicators: expo_rule.file_indicators.clone(),
                        });
                    }
                }
            }
            // Check for Next.js config files
            else if file_name.starts_with("next.config.") {
                if let Some(nextjs_rule) = rules.iter().find(|r| r.name == "Next.js") {
                    detected.push(DetectedTechnology {
                        name: nextjs_rule.name.clone(),
                        version: None,
                        category: nextjs_rule.category.clone(),
                        confidence: 1.0, // High confidence from config file
                        requires: nextjs_rule.requires.clone(),
                        conflicts_with: nextjs_rule.conflicts_with.clone(),
                        is_primary: nextjs_rule.is_primary_indicator,
                        file_indicators: nextjs_rule.file_indicators.clone(),
                    });
                }
            }
            // Check for React Native config files
            else if file_name == "react-native.config.js" {
                if let Some(rn_rule) = rules.iter().find(|r| r.name == "React Native") {
                    detected.push(DetectedTechnology {
                        name: rn_rule.name.clone(),
                        version: None,
                        category: rn_rule.category.clone(),
                        confidence: 1.0, // High confidence from config file
                        requires: rn_rule.requires.clone(),
                        conflicts_with: rn_rule.conflicts_with.clone(),
                        is_primary: rn_rule.is_primary_indicator,
                        file_indicators: rn_rule.file_indicators.clone(),
                    });
                }
            }
            // Check for Encore config files
            else if file_name == "encore.app" || file_name == "encore.service.ts" || file_name == "encore.service.js" {
                if let Some(encore_rule) = rules.iter().find(|r| r.name == "Encore") {
                    detected.push(DetectedTechnology {
                        name: encore_rule.name.clone(),
                        version: None,
                        category: encore_rule.category.clone(),
                        confidence: 1.0, // High confidence from config file
                        requires: encore_rule.requires.clone(),
                        conflicts_with: encore_rule.conflicts_with.clone(),
                        is_primary: encore_rule.is_primary_indicator,
                        file_indicators: encore_rule.file_indicators.clone(),
                    });
                }
            }
        }
    }
    
    if detected.is_empty() {
        None
    } else {
        Some(detected)
    }
}

/// New: Detect frameworks by analyzing project structure
fn detect_by_project_structure(language: &DetectedLanguage, rules: &[TechnologyRule]) -> Option<Vec<DetectedTechnology>> {
    let mut detected = Vec::new();
    let mut has_android_dir = false;
    let mut has_ios_dir = false;
    let mut has_pages_dir = false;
    let mut has_app_dir = false;
    let mut has_app_routes_dir = false;
    let mut has_encore_app_file = false;
    let mut has_encore_service_files = false;
    let mut has_app_json = false;
    let mut has_app_js_ts = false;
    let mut has_next_config = false;
    let mut has_tanstack_config = false;

    // Check project directories
    for file_path in &language.files {
        if let Some(parent) = file_path.parent() {
            let path_str = parent.to_string_lossy();
            let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            // Check for React Native structure
            if path_str.contains("android") {
                has_android_dir = true;
            } else if path_str.contains("ios") {
                has_ios_dir = true;
            }
            // Check for Next.js structure
            else if has_path_component(parent, "pages") {
                has_pages_dir = true;
            } else if has_path_component(parent, "app") && !file_name.contains("app.config") && !file_name.contains("encore.app") {
                has_app_dir = true;
            }
            // Check for TanStack Start structure
            else if has_app_routes(parent) {
                has_app_routes_dir = true;
            }
            // Check for Encore structure
            else if file_name == "encore.app" {
                has_encore_app_file = true;
            } else if file_name.contains("encore.service.") {
                has_encore_service_files = true;
            }
            // Check for Expo files
            else if file_name == "app.json" {
                has_app_json = true;
            } else if file_name == "App.js" || file_name == "App.tsx" {
                has_app_js_ts = true;
            }

            // Configs (need to be recorded so structure checks can require them)
            if file_name.starts_with("next.config.") {
                has_next_config = true;
            }
            if file_name == "app.config.ts" || file_name == "app.config.js" || file_name.starts_with("vinxi.config") {
                has_tanstack_config = true;
            }
        }
    }

    // Check if we have Expo dependencies
    let has_expo_deps = language.main_dependencies.iter().any(|dep| dep == "expo" || dep == "react-native");
    let has_next_dep = language.main_dependencies.iter().any(|dep| dep == "next" || dep.starts_with("next@"));
    let has_tanstack_dep = language.main_dependencies.iter().any(|dep| dep.contains("tanstack/react-start") || dep.contains("tanstack-start") || dep.contains("vinxi"));

    // Determine frameworks based on structure
    if has_encore_app_file || has_encore_service_files {
        // Likely Encore
        if let Some(encore_rule) = rules.iter().find(|r| r.name == "Encore") {
            detected.push(DetectedTechnology {
                name: encore_rule.name.clone(),
                version: None,
                category: encore_rule.category.clone(),
                confidence: 1.0, // High confidence from structure
                requires: encore_rule.requires.clone(),
                conflicts_with: encore_rule.conflicts_with.clone(),
                is_primary: encore_rule.is_primary_indicator,
                file_indicators: encore_rule.file_indicators.clone(),
            });
        }
    } else if has_app_routes_dir && (has_tanstack_dep || has_tanstack_config) {
        // Likely TanStack Start
        if let Some(tanstack_rule) = rules.iter().find(|r| r.name == "Tanstack Start") {
            detected.push(DetectedTechnology {
                name: tanstack_rule.name.clone(),
                version: None,
                category: tanstack_rule.category.clone(),
                confidence: 0.9, // Medium-high confidence from structure
                requires: tanstack_rule.requires.clone(),
                conflicts_with: tanstack_rule.conflicts_with.clone(),
                is_primary: tanstack_rule.is_primary_indicator,
                file_indicators: tanstack_rule.file_indicators.clone(),
            });
        }
    } else if (has_pages_dir || has_app_dir) && (has_next_dep || has_next_config) {
        // Likely Next.js
        if let Some(nextjs_rule) = rules.iter().find(|r| r.name == "Next.js") {
            detected.push(DetectedTechnology {
                name: nextjs_rule.name.clone(),
                version: None,
                category: nextjs_rule.category.clone(),
                confidence: 0.9, // Medium-high confidence from structure
                requires: nextjs_rule.requires.clone(),
                conflicts_with: nextjs_rule.conflicts_with.clone(),
                is_primary: nextjs_rule.is_primary_indicator,
                file_indicators: nextjs_rule.file_indicators.clone(),
            });
        }
    } else if (has_app_json || has_app_js_ts) && has_expo_deps {
        // Likely Expo (don't require Android/iOS directories for simpler detection)
        if let Some(expo_rule) = rules.iter().find(|r| r.name == "Expo") {
            detected.push(DetectedTechnology {
                name: expo_rule.name.clone(),
                version: None,
                category: expo_rule.category.clone(),
                confidence: 1.0, // High confidence from file structure and dependencies
                requires: expo_rule.requires.clone(),
                conflicts_with: expo_rule.conflicts_with.clone(),
                is_primary: expo_rule.is_primary_indicator,
                file_indicators: expo_rule.file_indicators.clone(),
            });
        }
    } else if has_android_dir && has_ios_dir {
        // Likely React Native
        if let Some(rn_rule) = rules.iter().find(|r| r.name == "React Native") {
            detected.push(DetectedTechnology {
                name: rn_rule.name.clone(),
                version: None,
                category: rn_rule.category.clone(),
                confidence: 0.9, // Medium-high confidence from structure
                requires: rn_rule.requires.clone(),
                conflicts_with: rn_rule.conflicts_with.clone(),
                is_primary: rn_rule.is_primary_indicator,
                file_indicators: rn_rule.file_indicators.clone(),
            });
        }
    }
    
    if detected.is_empty() {
        None
    } else {
        Some(detected)
    }
}

/// Returns true if any path component exactly matches the target (avoids substring false positives like "apps/")
fn has_path_component(path: &Path, target: &str) -> bool {
    path.components()
        .any(|c| c.as_os_str().to_string_lossy() == target)
}

/// Detects the canonical TanStack Start layout app/routes (component-level, not substring)
fn has_app_routes(path: &Path) -> bool {
    let components: Vec<String> = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();
    components.windows(2).any(|w| w[0] == "app" && w[1] == "routes")
}

/// New: Detect frameworks by analyzing source code patterns
fn detect_by_source_patterns(language: &DetectedLanguage, rules: &[TechnologyRule]) -> Option<Vec<DetectedTechnology>> {
    let mut detected = Vec::new();
    
    // Analyze files for usage patterns
    for file_path in &language.files {
        if let Ok(content) = std::fs::read_to_string(file_path) {
            // Check for Expo source patterns
            if content.contains("expo") && (content.contains("from 'expo'") || content.contains("import {") && content.contains("registerRootComponent")) {
                if let Some(expo_rule) = rules.iter().find(|r| r.name == "Expo") {
                    detected.push(DetectedTechnology {
                        name: expo_rule.name.clone(),
                        version: None,
                        category: expo_rule.category.clone(),
                        confidence: 0.8, // Higher confidence from more specific source patterns
                        requires: expo_rule.requires.clone(),
                        conflicts_with: expo_rule.conflicts_with.clone(),
                        is_primary: expo_rule.is_primary_indicator,
                        file_indicators: expo_rule.file_indicators.clone(),
                    });
                }
            }
            
            // Check for Next.js source patterns
            if content.contains("next/") {
                if let Some(nextjs_rule) = rules.iter().find(|r| r.name == "Next.js") {
                    detected.push(DetectedTechnology {
                        name: nextjs_rule.name.clone(),
                        version: None,
                        category: nextjs_rule.category.clone(),
                        confidence: 0.7, // Medium confidence from source patterns
                        requires: nextjs_rule.requires.clone(),
                        conflicts_with: nextjs_rule.conflicts_with.clone(),
                        is_primary: nextjs_rule.is_primary_indicator,
                        file_indicators: nextjs_rule.file_indicators.clone(),
                    });
                }
            }
            
            // Check for TanStack Router patterns
            if content.contains("@tanstack/react-router") && content.contains("createFileRoute") {
                if let Some(tanstack_rule) = rules.iter().find(|r| r.name == "Tanstack Start") {
                    detected.push(DetectedTechnology {
                        name: tanstack_rule.name.clone(),
                        version: None,
                        category: tanstack_rule.category.clone(),
                        confidence: 0.7, // Medium confidence from source patterns
                        requires: tanstack_rule.requires.clone(),
                        conflicts_with: tanstack_rule.conflicts_with.clone(),
                        is_primary: tanstack_rule.is_primary_indicator,
                        file_indicators: tanstack_rule.file_indicators.clone(),
                    });
                }
            }
            
            // Check for React Router patterns
            if content.contains("react-router") && content.contains("BrowserRouter") {
                if let Some(rr_rule) = rules.iter().find(|r| r.name == "React Router v7") {
                    detected.push(DetectedTechnology {
                        name: rr_rule.name.clone(),
                        version: None,
                        category: rr_rule.category.clone(),
                        confidence: 0.7, // Medium confidence from source patterns
                        requires: rr_rule.requires.clone(),
                        conflicts_with: rr_rule.conflicts_with.clone(),
                        is_primary: rr_rule.is_primary_indicator,
                        file_indicators: rr_rule.file_indicators.clone(),
                    });
                }
            }
        }
    }
    
    if detected.is_empty() {
        None
    } else {
        Some(detected)
    }
}

/// Enhanced detection that analyzes actual source files for technology usage patterns
fn detect_technologies_from_source_files(language: &DetectedLanguage, rules: &[TechnologyRule]) -> Option<Vec<DetectedTechnology>> {
    
    
    let mut detected = Vec::new();
    
    // Analyze files for usage patterns
    for file_path in &language.files {
        if let Ok(content) = fs::read_to_string(file_path) {
            // Analyze Drizzle ORM usage patterns
            if let Some(drizzle_confidence) = analyze_drizzle_usage(&content, file_path) {
                if let Some(drizzle_rule) = rules.iter().find(|r| r.name == "Drizzle ORM") {
                    detected.push(DetectedTechnology {
                        name: "Drizzle ORM".to_string(),
                        version: None,
                        category: TechnologyCategory::Database,
                        confidence: drizzle_confidence,
                        requires: vec![],
                        conflicts_with: vec![],
                        is_primary: false,
                        file_indicators: drizzle_rule.file_indicators.clone(),
                    });
                }
            }
            
            // Analyze Prisma usage patterns
            if let Some(prisma_confidence) = analyze_prisma_usage(&content, file_path) {
                if let Some(prisma_rule) = rules.iter().find(|r| r.name == "Prisma") {
                    detected.push(DetectedTechnology {
                        name: "Prisma".to_string(),
                        version: None,
                        category: TechnologyCategory::Database,
                        confidence: prisma_confidence,
                        requires: vec![],
                        conflicts_with: vec![],
                        is_primary: false,
                        file_indicators: prisma_rule.file_indicators.clone(),
                    });
                }
            }
            
            // Analyze Encore usage patterns
            if let Some(encore_confidence) = analyze_encore_usage(&content, file_path) {
                if let Some(encore_rule) = rules.iter().find(|r| r.name == "Encore") {
                    detected.push(DetectedTechnology {
                        name: "Encore".to_string(),
                        version: None,
                        category: TechnologyCategory::BackendFramework,
                        confidence: encore_confidence,
                        requires: vec![],
                        conflicts_with: vec![],
                        is_primary: true,
                        file_indicators: encore_rule.file_indicators.clone(),
                    });
                }
            }
            
            // Analyze Tanstack Start usage patterns
            if let Some(tanstack_confidence) = analyze_tanstack_start_usage(&content, file_path) {
                if let Some(tanstack_rule) = rules.iter().find(|r| r.name == "Tanstack Start") {
                    detected.push(DetectedTechnology {
                        name: "Tanstack Start".to_string(),
                        version: None,
                        category: TechnologyCategory::MetaFramework,
                        confidence: tanstack_confidence,
                        requires: vec!["React".to_string()],
                        conflicts_with: vec!["Next.js".to_string(), "React Router v7".to_string(), "SvelteKit".to_string(), "Nuxt.js".to_string()],
                        is_primary: true,
                        file_indicators: tanstack_rule.file_indicators.clone(),
                    });
                }
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
fn analyze_drizzle_usage(content: &str, file_path: &Path) -> Option<f32> {
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
fn analyze_prisma_usage(content: &str, file_path: &Path) -> Option<f32> {
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
fn analyze_encore_usage(content: &str, file_path: &Path) -> Option<f32> {
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
fn analyze_tanstack_start_usage(content: &str, file_path: &Path) -> Option<f32> {
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
            conflicts_with: vec!["Tanstack Start".to_string(), "React Router v7".to_string(), "SvelteKit".to_string(), "Nuxt.js".to_string(), "Expo".to_string()],
            is_primary_indicator: true,
            alternative_names: vec!["nextjs".to_string()],
            file_indicators: vec!["next.config.js".to_string(), "next.config.ts".to_string(), "pages/".to_string(), "app/".to_string()],
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
            file_indicators: vec!["app.config.ts".to_string(), "app.config.js".to_string(), "app/routes/".to_string(), "vite.config.ts".to_string()],
        },
        // React Router v7 as a framework (not just routing library) requires:
        // - @react-router/dev (the framework CLI) OR react-router.config.ts
        // - Just having react-router-dom is NOT enough (that's library usage)
        TechnologyRule {
            name: "React Router v7".to_string(),
            category: TechnologyCategory::MetaFramework,
            confidence: 0.95,
            // ONLY match the framework package, not just the routing library
            dependency_patterns: vec!["@react-router/dev".to_string(), "@react-router/node".to_string(), "@react-router/serve".to_string()],
            requires: vec!["React".to_string()],
            conflicts_with: vec!["Next.js".to_string(), "Tanstack Start".to_string(), "SvelteKit".to_string(), "Nuxt.js".to_string(), "React Native".to_string(), "Expo".to_string(), "Encore".to_string()],
            is_primary_indicator: true,
            alternative_names: vec!["remix".to_string()],
            file_indicators: vec!["react-router.config.ts".to_string(), "react-router.config.js".to_string()],
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
            file_indicators: vec!["svelte.config.js".to_string(), "svelte.config.ts".to_string()],
        },
        TechnologyRule {
            name: "Nuxt.js".to_string(),
            category: TechnologyCategory::MetaFramework,
            confidence: 0.95,
            dependency_patterns: vec!["nuxt".to_string()],
            requires: vec!["Vue.js".to_string()],
            conflicts_with: vec!["Next.js".to_string(), "Tanstack Start".to_string(), "React Router v7".to_string(), "SvelteKit".to_string()],
            is_primary_indicator: true,
            alternative_names: vec!["nuxtjs".to_string()],
            file_indicators: vec!["nuxt.config.ts".to_string(), "nuxt.config.js".to_string()],
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
            file_indicators: vec!["astro.config.mjs".to_string(), "astro.config.ts".to_string()],
        },
        TechnologyRule {
            name: "SolidStart".to_string(),
            category: TechnologyCategory::MetaFramework,
            confidence: 0.95,
            dependency_patterns: vec!["solid-start".to_string(), "@solidjs/start".to_string()],
            requires: vec!["SolidJS".to_string()],
            conflicts_with: vec!["Next.js".to_string(), "Tanstack Start".to_string(), "React Router v7".to_string(), "SvelteKit".to_string()],
            is_primary_indicator: true,
            alternative_names: vec![],
            file_indicators: vec!["app.config.ts".to_string(), "app.config.js".to_string()],
        },
        
        // MOBILE FRAMEWORKS (React Native/Expo)
        TechnologyRule {
            name: "React Native".to_string(),
            category: TechnologyCategory::FrontendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["react-native".to_string()],
            requires: vec!["React".to_string()],
            conflicts_with: vec!["Next.js".to_string(), "React Router v7".to_string(), "SvelteKit".to_string(), "Nuxt.js".to_string(), "Tanstack Start".to_string()],
            is_primary_indicator: true,
            alternative_names: vec!["reactnative".to_string()],
            file_indicators: vec!["react-native.config.js".to_string(), "android/".to_string(), "ios/".to_string()],
        },
        TechnologyRule {
            name: "Expo".to_string(),
            category: TechnologyCategory::MetaFramework,
            confidence: 1.0,
            dependency_patterns: vec!["expo".to_string(), "expo-router".to_string(), "@expo/vector-icons".to_string()],
            requires: vec!["React Native".to_string()],
            conflicts_with: vec!["Next.js".to_string(), "React Router v7".to_string(), "SvelteKit".to_string(), "Nuxt.js".to_string(), "Tanstack Start".to_string()],
            is_primary_indicator: true,
            alternative_names: vec![],
            file_indicators: vec!["app.json".to_string(), "app.config.js".to_string(), "app.config.ts".to_string()],
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
            file_indicators: vec!["angular.json".to_string(), "angular.cli.json".to_string()],
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
            file_indicators: vec!["svelte.config.js".to_string()],
        },
        
        // ROUTING LIBRARIES (Not frameworks! Just client-side routing)
        TechnologyRule {
            name: "React Router".to_string(),
            category: TechnologyCategory::Library(LibraryType::Routing),
            confidence: 0.85,
            // This is the routing LIBRARY, not the framework
            dependency_patterns: vec!["react-router-dom".to_string()],
            requires: vec!["React".to_string()],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
            file_indicators: vec![],
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
            file_indicators: vec![],
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
            file_indicators: vec![],
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
            file_indicators: vec![],
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
            file_indicators: vec![],
        },
        
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
            file_indicators: vec!["app.js".to_string(), "server.js".to_string()],
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
            file_indicators: vec!["fastify.config.js".to_string()],
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
            file_indicators: vec!["nest-cli.json".to_string()],
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
            file_indicators: vec![],
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
            file_indicators: vec![],
        },
        // Encore.ts - TypeScript backend framework
        // ONLY match encore.dev package, not just "encore" which is too generic
        TechnologyRule {
            name: "Encore".to_string(),
            category: TechnologyCategory::BackendFramework,
            confidence: 0.95,
            dependency_patterns: vec!["encore.dev".to_string()],
            requires: vec![],
            conflicts_with: vec!["Next.js".to_string(), "React Router v7".to_string(), "Tanstack Start".to_string()],
            is_primary_indicator: true,
            alternative_names: vec![],
            file_indicators: vec!["encore.app".to_string(), "encore.service.ts".to_string(), "encore.service.js".to_string()],
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
            file_indicators: vec![],
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
            file_indicators: vec![],
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
            file_indicators: vec![],
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
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "Sequelize".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.90,
            dependency_patterns: vec!["sequelize".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "TypeORM".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.90,
            dependency_patterns: vec!["typeorm".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "MikroORM".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.90,
            dependency_patterns: vec!["@mikro-orm/core".to_string(), "@mikro-orm/postgresql".to_string(), "@mikro-orm/mysql".to_string(), "@mikro-orm/sqlite".to_string(), "@mikro-orm/mongodb".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["mikro-orm".to_string()],
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "Mongoose".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.95,
            dependency_patterns: vec!["mongoose".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "Typegoose".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.90,
            dependency_patterns: vec!["@typegoose/typegoose".to_string()],
            requires: vec!["Mongoose".to_string()],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "Objection.js".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.90,
            dependency_patterns: vec!["objection".to_string()],
            requires: vec!["Knex.js".to_string()],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["objectionjs".to_string()],
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "Bookshelf".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.85,
            dependency_patterns: vec!["bookshelf".to_string()],
            requires: vec!["Knex.js".to_string()],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "Waterline".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.85,
            dependency_patterns: vec!["waterline".to_string(), "sails-mysql".to_string(), "sails-postgresql".to_string(), "sails-disk".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "Knex.js".to_string(),
            category: TechnologyCategory::Database,
            confidence: 0.85,
            dependency_patterns: vec!["knex".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["knexjs".to_string()],
            file_indicators: vec![],
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
            file_indicators: vec![],
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
            file_indicators: vec![],
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
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "WinterJS".to_string(),
            category: TechnologyCategory::Runtime,
            confidence: 0.95,
            dependency_patterns: vec!["winterjs".to_string(), "winter-js".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["winter.js".to_string()],
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "Cloudflare Workers".to_string(),
            category: TechnologyCategory::Runtime,
            confidence: 0.90,
            dependency_patterns: vec!["@cloudflare/workers-types".to_string(), "@cloudflare/vitest-pool-workers".to_string(), "wrangler".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["cloudflare-workers".to_string()],
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "Vercel Edge Runtime".to_string(),
            category: TechnologyCategory::Runtime,
            confidence: 0.90,
            dependency_patterns: vec!["@vercel/edge-runtime".to_string(), "@edge-runtime/vm".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec!["vercel-edge".to_string()],
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "Hermes".to_string(),
            category: TechnologyCategory::Runtime,
            confidence: 0.85,
            dependency_patterns: vec!["hermes-engine".to_string()],
            requires: vec!["React Native".to_string()],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "Electron".to_string(),
            category: TechnologyCategory::Runtime,
            confidence: 0.95,
            dependency_patterns: vec!["electron".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "Tauri".to_string(),
            category: TechnologyCategory::Runtime,
            confidence: 0.95,
            dependency_patterns: vec!["@tauri-apps/cli".to_string(), "@tauri-apps/api".to_string()],
            requires: vec![],
            conflicts_with: vec!["Electron".to_string()],
            is_primary_indicator: false,
            alternative_names: vec![],
            file_indicators: vec![],
        },
        TechnologyRule {
            name: "QuickJS".to_string(),
            category: TechnologyCategory::Runtime,
            confidence: 0.85,
            dependency_patterns: vec!["quickjs".to_string(), "quickjs-emscripten".to_string()],
            requires: vec![],
            conflicts_with: vec![],
            is_primary_indicator: false,
            alternative_names: vec![],
            file_indicators: vec![],
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
            file_indicators: vec![],
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
            file_indicators: vec![],
        },
    ]
} 
