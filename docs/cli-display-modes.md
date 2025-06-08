# CLI Display Modes

The `sync-ctl analyze` command now offers multiple display modes to present analysis results in different formats optimized for various use cases.

## Display Options

### 1. Matrix View (Default) - `--display matrix`

The matrix view provides a modern, compact dashboard that's easy to scan and compare projects side-by-side. This is the new default display mode.

```bash
sync-ctl analyze . --display matrix
# or simply
sync-ctl analyze .
```

**Example Output:**
```
═══════════════════════════════════════════════════════════════════════════════════════════════════
📊 PROJECT ANALYSIS DASHBOARD
═══════════════════════════════════════════════════════════════════════════════════════════════════

┌─ Architecture Overview ─────────────────────────────────────────────────────────────────────────┐
│ Type:               Monorepo (3 projects)                                                       │
│ Pattern:            Fullstack                                                                   │
│                     Full-stack app with frontend/backend separation                             │
└─────────────────────────────────────────────────────────────────────────────────────────────────┘

┌─ Technology Stack ──────────────────────────────────────────────────────────────────────────────┐
│ Languages:      TypeScript                                                                      │
│ Frameworks:     Encore, Tanstack Start                                                          │
│ Databases:      Drizzle ORM                                                                     │
└─────────────────────────────────────────────────────────────────────────────────────────────────┘

┌─ Projects Matrix ──────────────────────────────────────────────────────────────────────────────┐
│ ┌─────────────────┬──────────────┬───────────┬─────────────────┬───────┬────────┬──────────┐   │
│ │ Project         │ Type         │ Languages │ Main Tech       │ Ports │ Docker │ Deps     │   │
│ ├─────────────────┼──────────────┼───────────┼─────────────────┼───────┼────────┼──────────┤   │
│ │  backend        │ Backend        │ TypeScript│ Encore            │ 4000  │ ✓      │ 32   │   │
│ │  devops-agent   │ Infrastructure │ TypeScript │ -                │ -     │ ✗      │ 5    │   │
│ │  frontend       │ Frontend       │ TypeScript│ Tanstack Start    │ 3000  │ ✓      │ 123  │   │
│ └─────────────────┴──────────────┴───────────┴─────────────────┴───────┴────────┴──────────┘   │ 
└────────────────────────────────────────────────────────────────────────────────────────────────┘

┌─ Docker Infrastructure ─────────────────────────────────────────────────────────────────────────┐
│ Dockerfiles:              2                                                                     │
│ Compose Files:            2                                                                     │
│ Total Services:           5                                                                     │
│ Orchestration Patterns:   Microservices                                                         │
│ ────────────────────────────────────────────────────────────────────────────────────────────────│
│ Service Connectivity:                                                                           │
│   encore-postgres: 5431:5432                                                                    │
│   encore: 4000:8080 → encore-postgres                                                           │
│   intellitask-app: 3000:3000                                                                    │
└─────────────────────────────────────────────────────────────────────────────────────────────────┘

┌─ Analysis Metrics ─────────────────────────────────────────────────────────────────────────────┐
│  Duration: 57ms    Files: 294         Score: 87%         Version: 0.3.0                        │
└────────────────────────────────────────────────────────────────────────────────────────────────┘

═══════════════════════════════════════════════════════════════════════════════════════════════════
```

### 2. Summary View - `--display summary`

A brief overview of the analysis results, perfect for quick checks or CI/CD pipelines.

```bash
sync-ctl analyze . --display summary
```

**Example Output:**
```
▶ PROJECT ANALYSIS SUMMARY
──────────────────────────────────────────────────
│ Architecture: Monorepo (3 projects)
│ Pattern: Fullstack
│ Stack: TypeScript
│ Frameworks: Encore, Tanstack Start
│ Analysis Time: 57ms
│ Confidence: 87%
──────────────────────────────────────────────────
```

### 3. Detailed View (Legacy) - `--display detailed` or `-d`

The traditional verbose output with all details in a vertical layout. Useful when you need to see everything about each project.

```bash
sync-ctl analyze . --display detailed
# or for backward compatibility
sync-ctl analyze . -d
```

This produces the traditional long-form output with all details about each project.

### 4. JSON Output - `--json`

Machine-readable JSON output for integration with other tools or programmatic processing.

```bash
sync-ctl analyze . --json
```

## Choosing the Right Display Mode

- **Matrix View**: Best for daily use, comparing multiple projects, and getting a quick overview with key metrics
- **Summary View**: Ideal for CI/CD pipelines, scripts, or when you just need basic information
- **Detailed View**: Use when you need to see every detail about the analysis, including all dependencies, scripts, and configurations
- **JSON**: Perfect for integration with other tools, creating reports, or feeding data to dashboards

## Benefits of the New Matrix View

1. **Reduced Scrolling**: All important information fits on one screen
2. **Easy Comparison**: Projects are displayed side-by-side in a table
3. **Visual Hierarchy**: Box-drawing characters and colors create clear sections
4. **Key Metrics Focus**: Shows only the most important information by default
5. **Modern Appearance**: Clean, professional look with proper alignment
6. **LLM-Friendly**: The structured format is easy for AI assistants to parse and understand

## Color Coding

The matrix view uses colors strategically:
- **Blue**: Headers and structural elements
- **Yellow**: Important values and counts
- **Green**: Success indicators and positive metrics
- **Magenta**: Frameworks and technologies
- **Cyan**: Interactive elements and services
- **Red**: Error states or missing components

## Tips

- The matrix view automatically adjusts based on terminal width
- Use `--no-color` to disable colors if needed
- Pipe to `less` for scrolling in detailed view: `sync-ctl analyze . -d | less -R`
- Combine with `jq` for JSON processing: `sync-ctl analyze . --json | jq '.projects[].name'` 