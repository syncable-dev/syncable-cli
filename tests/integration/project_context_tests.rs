use syncable_cli::analyzer::{analyze_project_with_config, AnalysisConfig};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_full_project_context_analysis() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    
    // Create a realistic Node.js project
    create_nodejs_project(root);
    
    // Run full analysis
    let config = AnalysisConfig::default();
    let analysis = analyze_project_with_config(root, &config).unwrap();
    
    println!("=== Project Context Analysis Results ===");
    println!("Project Type: {:?}", analysis.project_type);
    
    // Verify Roadmap Requirement 1: Detect entry points and main files
    println!("\n✅ Entry Points Detected:");
    for entry in &analysis.entry_points {
        println!("  - File: {:?}", entry.file);
        println!("    Function: {:?}", entry.function);
        println!("    Command: {:?}", entry.command);
    }
    assert!(!analysis.entry_points.is_empty(), "Should detect entry points");
    
    // Verify Roadmap Requirement 2: Identify exposed ports and services
    println!("\n✅ Ports Detected:");
    for port in &analysis.ports {
        println!("  - Port {} ({:?}): {}", 
            port.number, 
            port.protocol, 
            port.description.as_ref().unwrap_or(&"No description".to_string())
        );
    }
    assert!(!analysis.ports.is_empty(), "Should detect exposed ports");
    
    // Verify Roadmap Requirement 3: Extract environment variables
    println!("\n✅ Environment Variables:");
    for env_var in &analysis.environment_variables {
        println!("  - {}: {} (default: {:?})", 
            env_var.name,
            if env_var.required { "required" } else { "optional" },
            env_var.default_value
        );
    }
    assert!(!analysis.environment_variables.is_empty(), "Should extract environment variables");
    
    // Verify Roadmap Requirement 4: Analyze build scripts and commands
    println!("\n✅ Build Scripts:");
    for script in &analysis.build_scripts {
        println!("  - {}: {} {}",
            script.name,
            script.command,
            if script.is_default { "(default)" } else { "" }
        );
    }
    assert!(!analysis.build_scripts.is_empty(), "Should analyze build scripts");
    
    // Verify Roadmap Requirement 5: Determine project type
    println!("\n✅ Project Type: {:?}", analysis.project_type);
    assert_ne!(analysis.project_type, syncable_cli::analyzer::ProjectType::Unknown, 
        "Should determine project type");
}

fn create_nodejs_project(root: &Path) {
    // Create package.json
    let package_json = r#"{
        "name": "test-express-app",
        "version": "1.0.0",
        "description": "Test Express application",
        "main": "server.js",
        "scripts": {
            "start": "node server.js",
            "dev": "nodemon server.js",
            "test": "jest",
            "build": "webpack --mode production",
            "lint": "eslint ."
        },
        "dependencies": {
            "express": "^4.18.0",
            "dotenv": "^16.0.0"
        },
        "devDependencies": {
            "nodemon": "^2.0.0",
            "jest": "^29.0.0",
            "eslint": "^8.0.0"
        }
    }"#;
    fs::write(root.join("package.json"), package_json).unwrap();
    
    // Create server.js
    let server_js = r#"
const express = require('express');
const dotenv = require('dotenv');

dotenv.config();

const app = express();

const PORT = process.env.PORT || 3000;
const API_KEY = process.env.API_KEY;
const DATABASE_URL = process.env.DATABASE_URL;
const NODE_ENV = process.env.NODE_ENV || 'development';

app.get('/', (req, res) => {
    res.json({ message: 'Hello World' });
});

app.listen(PORT, () => {
    console.log(`Server running on port ${PORT} in ${NODE_ENV} mode`);
});
    "#;
    fs::write(root.join("server.js"), server_js).unwrap();
    
    // Create .env file
    let env_file = r#"
# Server configuration
PORT=3000
NODE_ENV=development

# Database
DATABASE_URL=postgresql://localhost:5432/myapp

# API Keys
API_KEY=
SECRET_KEY=your-secret-key

# Feature flags
ENABLE_CACHE=true
DEBUG=false
    "#;
    fs::write(root.join(".env"), env_file).unwrap();
    
    // Create Dockerfile
    let dockerfile = r#"
FROM node:16-alpine
WORKDIR /app

COPY package*.json ./
RUN npm ci --only=production

COPY . .

ENV NODE_ENV=production
ENV PORT=8080

EXPOSE 8080

CMD ["node", "server.js"]
    "#;
    fs::write(root.join("Dockerfile"), dockerfile).unwrap();
    
    // Create docker-compose.yml
    let docker_compose = r#"
version: '3.8'

services:
  web:
    build: .
    ports:
      - "3000:8080"
    environment:
      - NODE_ENV=production
      - DATABASE_URL=postgres://user:pass@db:5432/myapp
    depends_on:
      - db
      
  db:
    image: postgres:14
    ports:
      - "5432:5432"
    environment:
      POSTGRES_USER: user
      POSTGRES_PASSWORD: pass
      POSTGRES_DB: myapp
    "#;
    fs::write(root.join("docker-compose.yml"), docker_compose).unwrap();
    
    // Create Makefile
    let makefile = r#"
.PHONY: build test run docker-build clean

build:
	npm install

test:
	npm test

run:
	npm start

docker-build:
	docker build -t myapp .

docker-run:
	docker-compose up

clean:
	rm -rf node_modules
	rm -rf dist
    "#;
    fs::write(root.join("Makefile"), makefile).unwrap();
}

#[test]
fn test_different_project_types() {
    // Test CLI tool detection
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    
    // Create a CLI tool project
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("Cargo.toml"), r#"
[package]
name = "my-cli"
version = "0.1.0"

[dependencies]
clap = "4.0"
    "#).unwrap();
    
    fs::write(root.join("src/main.rs"), r#"
use clap::Parser;

#[derive(Parser)]
struct Cli {
    name: String,
}

fn main() {
    let args = Cli::parse();
    println!("Hello, {}", args.name);
}
    "#).unwrap();
    
    let config = AnalysisConfig::default();
    let analysis = analyze_project_with_config(root, &config).unwrap();
    
    println!("\n=== CLI Tool Project Analysis ===");
    println!("Project Type: {:?}", analysis.project_type);
    assert_eq!(analysis.project_type, syncable_cli::analyzer::ProjectType::CliTool);
}

#[test]
fn test_real_world_scenarios() {
    // Test with a project that has multiple configuration sources
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    
    // Create Python FastAPI project
    fs::write(root.join("requirements.txt"), r#"
fastapi==0.100.0
uvicorn==0.23.0
sqlalchemy==2.0.0
redis==4.5.0
    "#).unwrap();
    
    fs::write(root.join("main.py"), r#"
import os
from fastapi import FastAPI
from sqlalchemy import create_engine

app = FastAPI()

# Configuration from environment
DATABASE_URL = os.environ.get("DATABASE_URL", "sqlite:///./test.db")
REDIS_URL = os.getenv("REDIS_URL", "redis://localhost:6379")
SECRET_KEY = os.environ["SECRET_KEY"]  # Required
PORT = int(os.getenv("PORT", "8000"))

@app.get("/")
def read_root():
    return {"Hello": "World"}

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=PORT)
    "#).unwrap();
    
    // Create setup.py with console scripts
    fs::write(root.join("setup.py"), r#"
from setuptools import setup

setup(
    name="myapi",
    version="1.0.0",
    py_modules=["main"],
    entry_points={
        'console_scripts': [
            'myapi=main:run_server',
            'myapi-migrate=migrate:main',
        ],
    },
)
    "#).unwrap();
    
    let config = AnalysisConfig::default();
    let analysis = analyze_project_with_config(root, &config).unwrap();
    
    println!("\n=== FastAPI Project Analysis ===");
    println!("Detected {} entry points", analysis.entry_points.len());
    println!("Detected {} environment variables", analysis.environment_variables.len());
    println!("Detected {} ports", analysis.ports.len());
    
    // Verify comprehensive detection
    assert!(analysis.environment_variables.iter().any(|ev| ev.name == "DATABASE_URL"));
    assert!(analysis.environment_variables.iter().any(|ev| ev.name == "SECRET_KEY" && ev.required));
    assert!(analysis.ports.iter().any(|p| p.number == 8000));
    assert_eq!(analysis.project_type, syncable_cli::analyzer::ProjectType::ApiService);
} 