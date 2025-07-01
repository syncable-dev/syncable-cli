use crate::analyzer::{DetectedTechnology, EntryPoint, Port, Protocol};
use crate::error::Result;
use std::collections::HashSet;
use std::path::{Path};

/// Analyzes technology-specific configurations
pub(crate) fn analyze_technology_specifics(
    technology: &DetectedTechnology,
    root: &Path,
    entry_points: &mut Vec<EntryPoint>,
    ports: &mut HashSet<Port>,
) -> Result<()> {
    match technology.name.as_str() {
        "Next.js" => {
            // Next.js typically runs on port 3000
            ports.insert(Port {
                number: 3000,
                protocol: Protocol::Http,
                description: Some("Next.js development server".to_string()),
            });

            // Look for pages directory
            let pages_dir = root.join("pages");
            if pages_dir.is_dir() {
                entry_points.push(EntryPoint {
                    file: pages_dir,
                    function: None,
                    command: Some("npm run dev".to_string()),
                });
            }
        }
        "Express" | "Fastify" | "Koa" | "Hono" | "Elysia" => {
            // Common Node.js web framework ports
            ports.insert(Port {
                number: 3000,
                protocol: Protocol::Http,
                description: Some(format!("{} server", technology.name)),
            });
        }
        "Encore" => {
            // Encore development server typically runs on port 4000
            ports.insert(Port {
                number: 4000,
                protocol: Protocol::Http,
                description: Some("Encore development server".to_string()),
            });
        }
        "Astro" => {
            // Astro development server typically runs on port 3000 or 4321
            ports.insert(Port {
                number: 4321,
                protocol: Protocol::Http,
                description: Some("Astro development server".to_string()),
            });
        }
        "SvelteKit" => {
            // SvelteKit development server typically runs on port 5173
            ports.insert(Port {
                number: 5173,
                protocol: Protocol::Http,
                description: Some("SvelteKit development server".to_string()),
            });
        }
        "Nuxt.js" => {
            // Nuxt.js development server typically runs on port 3000
            ports.insert(Port {
                number: 3000,
                protocol: Protocol::Http,
                description: Some("Nuxt.js development server".to_string()),
            });
        }
        "Tanstack Start" => {
            // Modern React framework typically runs on port 3000
            ports.insert(Port {
                number: 3000,
                protocol: Protocol::Http,
                description: Some(format!("{} development server", technology.name)),
            });
        }
        "React Router v7" => {
            // React Router v7 development server typically runs on port 5173
            ports.insert(Port {
                number: 5173,
                protocol: Protocol::Http,
                description: Some("React Router v7 development server".to_string()),
            });
        }
        "Django" => {
            ports.insert(Port {
                number: 8000,
                protocol: Protocol::Http,
                description: Some("Django development server".to_string()),
            });
        }
        "Flask" | "FastAPI" => {
            ports.insert(Port {
                number: 5000,
                protocol: Protocol::Http,
                description: Some(format!("{} server", technology.name)),
            });
        }
        "Spring Boot" => {
            ports.insert(Port {
                number: 8080,
                protocol: Protocol::Http,
                description: Some("Spring Boot server".to_string()),
            });
        }
        "Actix Web" | "Rocket" => {
            ports.insert(Port {
                number: 8080,
                protocol: Protocol::Http,
                description: Some(format!("{} server", technology.name)),
            });
        }
        _ => {}
    }

    Ok(())
} 