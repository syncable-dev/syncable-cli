# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.35.0](https://github.com/syncable-dev/syncable-cli/compare/v0.34.1...v0.35.0) - 2026-02-05

### Added

- vendor ag-ui-core and ag-ui-server crates
- new availability feature for hetzner deployment through agent. automatically searching available regions/machine types dynamically
- *(agent)* add list_hetzner_availability tool, require dynamic data for Hetzner
- early ag-ui implementation with test along
- *(hetzner)* remove hardcoded data, require dynamic API fetching
- *(wizard)* integrate dynamic Hetzner availability into deployment wizard
- *(hetzner)* add dynamic availability API for smart resource selection
- *(23-01)* wire CopilotKit provider and navigation
- *(23-01)* create agent chat route with CopilotKit
- *(23-01)* create CopilotKit provider wrapper
- *(22-01)* wire processor to server startup
- *(22-01)* implement message processing loop
- *(22-01)* create AgentProcessor module with session management
- *(21-01)* add POST /message endpoint
- *(21-01)* handle WebSocket incoming messages
- *(21-01)* add message channel to ServerState
- *(19-01)* add container deployment configurations
- *(18-01)* add agent command for headless AG-UI server mode
- *(17-01)* emit step/thinking events during agent processing
- *(16-01)* add interrupt methods to EventBridge for human-in-the-loop
- *(15-01)* add AG-UI state synchronization
- *(14-01)* wire LLM response handling to AG-UI EventBridge
- *(13-01)* connect ToolDisplayHook to EventBridge for tool events
- *(12-01)* add --ag-ui flag for frontend connectivity

### Fixed

- *(hetzner)* use availability API for real-time capacity data
- *(hetzner)* use /api/v1/cloud-runner/hetzner/options endpoint
- *(23-01)* use CopilotChat component instead of headless API

### Other

- Merge pull request #287 from syncable-dev/develop
- Merge branch 'develop' of github.com:syncable-dev/syncable-cli into develop
- *(23-01)* add CopilotKit dependencies
- *(20-01)* add AG-UI server integration tests

## [0.34.1](https://github.com/syncable-dev/syncable-cli/releases/tag/v0.34.1) - 2026-01-22

### Other

- release v0.34.1
- release v0.34.0
- release v0.34.0

## [0.34.1](https://github.com/syncable-dev/syncable-cli/compare/v0.34.0...v0.34.1) - 2026-01-21

### Other

- release v0.34.0
- release v0.34.0

## [0.34.0](https://github.com/syncable-dev/syncable-cli/releases/tag/v0.34.0) - 2026-01-20

### Added

- *(11.3-03)* add DeployServiceTool for conversational deployment
- *(11.3-02)* add deployment recommendation engine
- *(11.3-01)* add infrastructure presence detection
- *(11.3-01)* add health endpoint detection
- *(11.3-01)* add PortSource enum for source-based port tracking
- *(11.1-01)* fix CloudRunnerConfig to use provider-nested structure
- *(wizard)* add smart repository connection to deploy flow
- *(11-01)* add GitHub integration API types and methods
- *(62.2-01)* integrate Dockerfile selection into wizard
- *(62.2-01)* add Dockerfile selection wizard step
- *(62.1-02)* add deploy new-env command with wizard
- *(62.1-02)* add EnvCommand to CLI with list and select
- *(62.1-01)* add environment fields to PlatformSession
- *(62.1-01)* add Environment type and API methods
- *(61-01)* add is_available to list_deployment_capabilities tool
- *(61-01)* show Coming Soon for unavailable providers in wizard
- *(61-01)* add Scaleway, Cyso providers and is_available method
- *(60-01)* cross-reference analyze_codebase in analyze_project next_steps
- *(60-01)* register AnalyzeCodebaseTool in platform module
- *(60-01)* create AnalyzeCodebaseTool for comprehensive analysis
- *(59-02)* create ProvisionRegistryTool and register tools
- *(59-02)* create CreateDeploymentConfigTool for agent
- *(59-02)* add create_deployment_config API method
- *(59-01)* create ListDeploymentCapabilitiesTool and register tools
- *(59-01)* create AnalyzeProjectTool for deployment discovery
- *(58-01)* integrate registry provisioning into wizard orchestrator
- *(58-01)* create registry provisioning wizard step
- *(58-01)* add registry provisioning types and API methods
- *(57-03)* CLI deploy wizard command integration
- *(57-03)* wizard orchestration
- *(57-03)* service configuration form
- *(57-02)* implement registry selection step
- *(57-02)* implement cluster selection step
- *(57-02)* implement target selection step
- *(57-01)* implement provider selection prompt
- *(57-01)* implement provider status aggregation
- *(57-01)* create wizard module structure
- *(56-01)* add CLI wizard deployment config types
- *(analyzer)* add dockerfile discovery for deployment wizard
- *(platform)* add cluster and registry API methods
- *(46-01)* add API connection health check
- *(46-01)* add actionable suggestions to API errors
- *(46-01)* add retry logic for transient API failures
- *(45-01)* add platform context to input prompt
- *(45-01)* add platform context to welcome banner
- *(44-01)* wire up Project and Org commands in main.rs
- *(44-01)* implement Project and Org command handlers
- *(44-01)* add Project and Org command definitions
- *(43-01)* create GetServiceLogsTool
- *(43-01)* add log types and API method
- *(42-01)* register deployment tools with agent
- *(42-01)* create deployment tools
- *(42-01)* add deployment types and API methods
- *(41-01)* register provider connection tools
- *(41-01)* create provider connection tools
- *(41-01)* add provider connection check to API client
- *(40-01)* register platform tools with agent
- *(40-01)* create platform listing and selection tools
- *(39-01)* create platform API client module
- *(38-01)* wire session loading into agent startup
- *(38-01)* create platform session module

### Fixed

- *(11.3-01)* enforce human-in-the-loop for deployment changes
- *(11.3-01)* add is_public parameter with safe default (false)
- *(11.3-01)* prevent agent from polling deployment status in infinite loop
- *(11.3-01)* detect correct repository from local git remote
- *(11.3-01)* derive dockerfile paths relative to repo root for Cloud Runner
- *(deploy)* use paths relative to analyzed dir, not project root
- *(deploy)* match manual wizard dockerfile/context path handling
- *(deploy)* correct dockerfile path derivation for subdirectory deployments
- *(prompt)* reduce agent narration of internal reasoning
- *(deploy-status)* check actual service readiness for Cloud Runner
- *(agent)* register CreateDeploymentConfigTool and DeployServiceTool
- *(agent)* register ListDeploymentCapabilitiesTool in agent
- *(api)* use working endpoint for check_provider_connection
- *(api)* wrap get_optional responses in GenericResponse
- *(deploy)* add duplicate detection and environment display to DeployServiceTool
- *(wizard)* use build_context + filename for dockerfile path
- *(wizard)* use full dockerfile path for Docker build
- dockerfile path relative to build context + add deploy status command
- *(api)* correct trigger deployment response parsing
- *(api)* correct deployment config API response parsing
- *(62.1-02)* correct ArtifactRegistry cloudProvider field name
- *(62.1-02)* correct environment API endpoint and field names
- *(62-01)* make deploy wizard the default when no subcommand provided
- detect provider connection from cloud credentials, not resources
- *(api)* unwrap GenericResponse wrapper in platform API client

### Other

- release v0.34.0
- release v0.34.0
- Merge pull request #279 from syncable-dev/develop
- add verbose logging for deployment config request
- *(wizard)* add debug logging for deployment trigger
- *(62-01)* fix clippy never_loop warnings in wizard orchestrator

## [0.34.0](https://github.com/syncable-dev/syncable-cli/releases/tag/v0.34.0) - 2026-01-20

### Added

- *(11.3-03)* add DeployServiceTool for conversational deployment
- *(11.3-02)* add deployment recommendation engine
- *(11.3-01)* add infrastructure presence detection
- *(11.3-01)* add health endpoint detection
- *(11.3-01)* add PortSource enum for source-based port tracking
- *(11.1-01)* fix CloudRunnerConfig to use provider-nested structure
- *(wizard)* add smart repository connection to deploy flow
- *(11-01)* add GitHub integration API types and methods
- *(62.2-01)* integrate Dockerfile selection into wizard
- *(62.2-01)* add Dockerfile selection wizard step
- *(62.1-02)* add deploy new-env command with wizard
- *(62.1-02)* add EnvCommand to CLI with list and select
- *(62.1-01)* add environment fields to PlatformSession
- *(62.1-01)* add Environment type and API methods
- *(61-01)* add is_available to list_deployment_capabilities tool
- *(61-01)* show Coming Soon for unavailable providers in wizard
- *(61-01)* add Scaleway, Cyso providers and is_available method
- *(60-01)* cross-reference analyze_codebase in analyze_project next_steps
- *(60-01)* register AnalyzeCodebaseTool in platform module
- *(60-01)* create AnalyzeCodebaseTool for comprehensive analysis
- *(59-02)* create ProvisionRegistryTool and register tools
- *(59-02)* create CreateDeploymentConfigTool for agent
- *(59-02)* add create_deployment_config API method
- *(59-01)* create ListDeploymentCapabilitiesTool and register tools
- *(59-01)* create AnalyzeProjectTool for deployment discovery
- *(58-01)* integrate registry provisioning into wizard orchestrator
- *(58-01)* create registry provisioning wizard step
- *(58-01)* add registry provisioning types and API methods
- *(57-03)* CLI deploy wizard command integration
- *(57-03)* wizard orchestration
- *(57-03)* service configuration form
- *(57-02)* implement registry selection step
- *(57-02)* implement cluster selection step
- *(57-02)* implement target selection step
- *(57-01)* implement provider selection prompt
- *(57-01)* implement provider status aggregation
- *(57-01)* create wizard module structure
- *(56-01)* add CLI wizard deployment config types
- *(analyzer)* add dockerfile discovery for deployment wizard
- *(platform)* add cluster and registry API methods
- *(46-01)* add API connection health check
- *(46-01)* add actionable suggestions to API errors
- *(46-01)* add retry logic for transient API failures
- *(45-01)* add platform context to input prompt
- *(45-01)* add platform context to welcome banner
- *(44-01)* wire up Project and Org commands in main.rs
- *(44-01)* implement Project and Org command handlers
- *(44-01)* add Project and Org command definitions
- *(43-01)* create GetServiceLogsTool
- *(43-01)* add log types and API method
- *(42-01)* register deployment tools with agent
- *(42-01)* create deployment tools
- *(42-01)* add deployment types and API methods
- *(41-01)* register provider connection tools
- *(41-01)* create provider connection tools
- *(41-01)* add provider connection check to API client
- *(40-01)* register platform tools with agent
- *(40-01)* create platform listing and selection tools
- *(39-01)* create platform API client module
- *(38-01)* wire session loading into agent startup
- *(38-01)* create platform session module

### Fixed

- *(11.3-01)* enforce human-in-the-loop for deployment changes
- *(11.3-01)* add is_public parameter with safe default (false)
- *(11.3-01)* prevent agent from polling deployment status in infinite loop
- *(11.3-01)* detect correct repository from local git remote
- *(11.3-01)* derive dockerfile paths relative to repo root for Cloud Runner
- *(deploy)* use paths relative to analyzed dir, not project root
- *(deploy)* match manual wizard dockerfile/context path handling
- *(deploy)* correct dockerfile path derivation for subdirectory deployments
- *(prompt)* reduce agent narration of internal reasoning
- *(deploy-status)* check actual service readiness for Cloud Runner
- *(agent)* register CreateDeploymentConfigTool and DeployServiceTool
- *(agent)* register ListDeploymentCapabilitiesTool in agent
- *(api)* use working endpoint for check_provider_connection
- *(api)* wrap get_optional responses in GenericResponse
- *(deploy)* add duplicate detection and environment display to DeployServiceTool
- *(wizard)* use build_context + filename for dockerfile path
- *(wizard)* use full dockerfile path for Docker build
- dockerfile path relative to build context + add deploy status command
- *(api)* correct trigger deployment response parsing
- *(api)* correct deployment config API response parsing
- *(62.1-02)* correct ArtifactRegistry cloudProvider field name
- *(62.1-02)* correct environment API endpoint and field names
- *(62-01)* make deploy wizard the default when no subcommand provided
- detect provider connection from cloud credentials, not resources
- *(api)* unwrap GenericResponse wrapper in platform API client

### Other

- release v0.34.0
- Merge pull request #279 from syncable-dev/develop
- add verbose logging for deployment config request
- *(wizard)* add debug logging for deployment trigger
- *(62-01)* fix clippy never_loop warnings in wizard orchestrator

## [0.34.0](https://github.com/syncable-dev/syncable-cli/compare/v0.33.0...v0.34.0) - 2026-01-20

### Added

- *(11.3-03)* add DeployServiceTool for conversational deployment
- *(11.3-02)* add deployment recommendation engine
- *(11.3-01)* add infrastructure presence detection
- *(11.3-01)* add health endpoint detection
- *(11.3-01)* add PortSource enum for source-based port tracking
- *(11.1-01)* fix CloudRunnerConfig to use provider-nested structure
- *(wizard)* add smart repository connection to deploy flow
- *(11-01)* add GitHub integration API types and methods
- *(62.2-01)* integrate Dockerfile selection into wizard
- *(62.2-01)* add Dockerfile selection wizard step
- *(62.1-02)* add deploy new-env command with wizard
- *(62.1-02)* add EnvCommand to CLI with list and select
- *(62.1-01)* add environment fields to PlatformSession
- *(62.1-01)* add Environment type and API methods
- *(61-01)* add is_available to list_deployment_capabilities tool
- *(61-01)* show Coming Soon for unavailable providers in wizard
- *(61-01)* add Scaleway, Cyso providers and is_available method
- *(60-01)* cross-reference analyze_codebase in analyze_project next_steps
- *(60-01)* register AnalyzeCodebaseTool in platform module
- *(60-01)* create AnalyzeCodebaseTool for comprehensive analysis
- *(59-02)* create ProvisionRegistryTool and register tools
- *(59-02)* create CreateDeploymentConfigTool for agent
- *(59-02)* add create_deployment_config API method
- *(59-01)* create ListDeploymentCapabilitiesTool and register tools
- *(59-01)* create AnalyzeProjectTool for deployment discovery
- *(58-01)* integrate registry provisioning into wizard orchestrator
- *(58-01)* create registry provisioning wizard step
- *(58-01)* add registry provisioning types and API methods
- *(57-03)* CLI deploy wizard command integration
- *(57-03)* wizard orchestration
- *(57-03)* service configuration form
- *(57-02)* implement registry selection step
- *(57-02)* implement cluster selection step
- *(57-02)* implement target selection step
- *(57-01)* implement provider selection prompt
- *(57-01)* implement provider status aggregation
- *(57-01)* create wizard module structure
- *(56-01)* add CLI wizard deployment config types
- *(analyzer)* add dockerfile discovery for deployment wizard
- *(platform)* add cluster and registry API methods
- *(46-01)* add API connection health check
- *(46-01)* add actionable suggestions to API errors
- *(46-01)* add retry logic for transient API failures
- *(45-01)* add platform context to input prompt
- *(45-01)* add platform context to welcome banner
- *(44-01)* wire up Project and Org commands in main.rs
- *(44-01)* implement Project and Org command handlers
- *(44-01)* add Project and Org command definitions
- *(43-01)* create GetServiceLogsTool
- *(43-01)* add log types and API method
- *(42-01)* register deployment tools with agent
- *(42-01)* create deployment tools
- *(42-01)* add deployment types and API methods
- *(41-01)* register provider connection tools
- *(41-01)* create provider connection tools
- *(41-01)* add provider connection check to API client
- *(40-01)* register platform tools with agent
- *(40-01)* create platform listing and selection tools
- *(39-01)* create platform API client module
- *(38-01)* wire session loading into agent startup
- *(38-01)* create platform session module

### Fixed

- *(11.3-01)* enforce human-in-the-loop for deployment changes
- *(11.3-01)* add is_public parameter with safe default (false)
- *(11.3-01)* prevent agent from polling deployment status in infinite loop
- *(11.3-01)* detect correct repository from local git remote
- *(11.3-01)* derive dockerfile paths relative to repo root for Cloud Runner
- *(deploy)* use paths relative to analyzed dir, not project root
- *(deploy)* match manual wizard dockerfile/context path handling
- *(deploy)* correct dockerfile path derivation for subdirectory deployments
- *(prompt)* reduce agent narration of internal reasoning
- *(deploy-status)* check actual service readiness for Cloud Runner
- *(agent)* register CreateDeploymentConfigTool and DeployServiceTool
- *(agent)* register ListDeploymentCapabilitiesTool in agent
- *(api)* use working endpoint for check_provider_connection
- *(api)* wrap get_optional responses in GenericResponse
- *(deploy)* add duplicate detection and environment display to DeployServiceTool
- *(wizard)* use build_context + filename for dockerfile path
- *(wizard)* use full dockerfile path for Docker build
- dockerfile path relative to build context + add deploy status command
- *(api)* correct trigger deployment response parsing
- *(api)* correct deployment config API response parsing
- *(62.1-02)* correct ArtifactRegistry cloudProvider field name
- *(62.1-02)* correct environment API endpoint and field names
- *(62-01)* make deploy wizard the default when no subcommand provided
- detect provider connection from cloud credentials, not resources
- *(api)* unwrap GenericResponse wrapper in platform API client

### Other

- Merge pull request #279 from syncable-dev/develop
- add verbose logging for deployment config request
- *(wizard)* add debug logging for deployment trigger
- *(62-01)* fix clippy never_loop warnings in wizard orchestrator

## [0.33.0](https://github.com/syncable-dev/syncable-cli/compare/v0.32.1...v0.33.0) - 2026-01-15

### Added

- matrix ui upgrade for better view and visibility
- *(07-03)* session persistence with full context restore
- *(06-03)* improve k8s_costs tool with error patterns
- *(06-02)* improve prometheus_connect tool with error patterns
- *(06-01)* improve k8s_optimize tool with error patterns
- *(05-04)* improve dclint tool with error patterns and tests
- *(05-03)* improve kubelint tool with error patterns and tests
- *(05-02)* improve helmlint tool with error patterns and tests
- *(05-01)* improve hadolint tool with error patterns and tests
- *(04-03)* add analyze tool edge case handling
- *(04-03)* improve analyze tool definition
- *(04-02)* add file_ops edge case handling
- *(04-02)* improve file_ops path validation error messages
- *(04-02)* improve file_ops tool definitions
- *(04-01)* improve shell tool definition and rejection messages
- *(04-01)* expand shell command allowlist with categories
- *(03-03)* update core tools with response formatting
- *(03-03)* create response formatting utilities
- *(03-02)* add error module to tools with documentation
- *(03-02)* create common error utilities module

### Fixed

- *(fomatting)* missing formatting
- *(ci)* bump MSRV to 1.88 for AWS SDK compatibility
- *(ci)* bump MSRV to 1.87 and ignore transitive security advisories
- *(07-02)* preserve context during history truncation

### Other

- Merge pull request #277 from syncable-dev/develop
- small fixes
- *(09-02)* add tests to untested tool files
- *(08-02)* add tests for input.rs and autocomplete.rs
- *(04-01)* add shell tool allowlist tests
- *(03-03)* document response patterns in mod.rs
- *(03-02)* update high-priority tools with error utilities
- *(02-04)* extract UI helpers to session/ui.rs
- *(02-03)* update session/mod.rs to delegate to commands
- *(02-03)* create commands.rs with all handle_* methods
- *(02-02)* extract provider logic into providers.rs submodule
- *(02-01)* create session submodule structure and extract plan_mode
- *(01-02)* create testing protocol for all 28 tools
- *(01)* create phase 1 audit & triage plans

## [0.32.1](https://github.com/syncable-dev/syncable-cli/compare/v0.32.0...v0.32.1) - 2026-01-11

### Added

- small fixes, truncation for docker output, default bedrock model fix, and lastly shell error fixed

## [0.32.0](https://github.com/syncable-dev/syncable-cli/compare/v0.31.1...v0.32.0) - 2026-01-09

### Added

- updated agent store logic to better fetch and manage outputs
- upgrade rig-core to 0.28 and fix OpenAI Responses API multi-turn

### Fixed

- *(agent)* [**breaking**] use monorepo analyzer to detect ALL projects instead of flat analysis
- resolve clippy errors and failing tests for CI

### Other

- Merge pull request #270 from syncable-dev/develop
- Merge branch 'develop' of github.com:syncable-dev/syncable-cli into develop

## [0.31.1](https://github.com/syncable-dev/syncable-cli/compare/v0.31.0...v0.31.1) - 2026-01-06

### Other

- update Cargo.lock dependencies

## [0.31.0](https://github.com/syncable-dev/syncable-cli/compare/v0.30.1...v0.31.0) - 2026-01-01

### Added

- updated docs and agent resume querry

## [0.30.1](https://github.com/syncable-dev/syncable-cli/compare/v0.30.0...v0.30.1) - 2025-12-31

### Added

- fixed fmt / clappy issues in ci pipeline
- added authentication, for agent usage
- rant fmt and lint check

## [0.30.0](https://github.com/syncable-dev/syncable-cli/compare/v0.29.5...v0.30.0) - 2025-12-31

### Added

- updated cli to include auth

### Other

- Merge pull request #261 from syncable-dev/develop

## [0.29.5](https://github.com/syncable-dev/syncable-cli/compare/v0.29.4...v0.29.5) - 2025-12-30

### Added

- updated gif

### Other

- Merge pull request #259 from syncable-dev/develop

## [0.29.4](https://github.com/syncable-dev/syncable-cli/compare/v0.29.3...v0.29.4) - 2025-12-29

### Added

- *(linters)* add native Rust kubelint and helmlint tools
- updated rust crate or hashtags

### Other

- Merge pull request #250 from syncable-dev/develop

## [0.29.3](https://github.com/syncable-dev/syncable-cli/compare/v0.29.2...v0.29.3) - 2025-12-27

### Added

- removed main CI pipeline due to using releaze, and we only want to validate develop anyway
- added prompts reference, so catching up correctly plans pointed continuation requests

## [0.29.2](https://github.com/syncable-dev/syncable-cli/compare/v0.29.1...v0.29.2) - 2025-12-27

### Other

- Merge pull request #246 from syncable-dev/develop
- Merge branch 'develop' of github.com:syncable-dev/syncable-cli into develop
- *(bedrock)* inline rig-bedrock module for crates.io compatibility

## [0.29.1](https://github.com/syncable-dev/syncable-cli/compare/v0.29.0...v0.29.1) - 2025-12-27

### Other

- update Cargo.lock dependencies

## [0.29.0](https://github.com/syncable-dev/syncable-cli/compare/v0.28.1...v0.29.0) - 2025-12-27

### Fixed

- *(clippy)* resolve all clippy warnings across codebase

### Other

- fix print_with_newline clippy lint and format code

## [0.28.1](https://github.com/syncable-dev/syncable-cli/compare/v0.28.0...v0.28.1) - 2025-12-27

### Added

- add CI workflow, trust badges, and fix Bedrock extended thinking
- updated README with gif

### Fixed

- *(ci)* remove recursive clippy alias that broke CI
- *(ci)* ignore flaky integration test and fix doctests
- fix flaky tests and extract_environment_from_filename bug
- clone PathBuf to fix Windows build error
- *(ci)* add permissions for security audit and ignore unmaintained warnings
- *(ci)* override target-cpu=native that breaks macOS CI
- *(ci)* remove recursive cargo fmt alias that broke CI

### Other

- Merge pull request #240 from syncable-dev/develop
- format dclint tool files
- add docker-compose-linter attribution
- run cargo fmt --all
- add demo GIF to README for better conversions

## [0.28.0](https://github.com/syncable-dev/syncable-cli/compare/v0.27.2...v0.28.0) - 2025-12-26

### Added

- updated .gitignore
- *(agent)* add plan mode, plan resumption, and context overflow fixes

### Other

- Merge pull request #238 from syncable-dev/develop
- bug(wrong ref for rig-bedrocks) wrong referenced rig-bedrock package
- Merge branch 'develop' of github.com:syncable-dev/syncable-cli into develop

## [0.27.2](https://github.com/syncable-dev/syncable-cli/compare/v0.27.1...v0.27.2) - 2025-12-23

### Other

- Merge pull request #232 from syncable-dev/develop
- Merge pull request #225 from syncable-dev/dependabot/cargo/develop/crossterm-0.29.0

## [0.27.1](https://github.com/syncable-dev/syncable-cli/compare/v0.27.0...v0.27.1) - 2025-12-23

### Other

- Merge pull request #230 from syncable-dev/develop
- Merge pull request #226 from syncable-dev/dependabot/cargo/develop/serde_json-1.0.146
- Merge pull request #227 from syncable-dev/dependabot/cargo/develop/rustyline-17.0.2
- *(deps)* bump rustyline from 15.0.0 to 17.0.2

## [0.27.0](https://github.com/syncable-dev/syncable-cli/compare/v0.26.1...v0.27.0) - 2025-12-23

### Added

- *(agent)* add extended thinking, conversation compaction, and UI improvements

### Other

- Merge branch 'main' into develop

## [0.26.1](https://github.com/syncable-dev/syncable-cli/compare/v0.26.0...v0.26.1) - 2025-12-21

### Added

- *(hadolint)* add native Rust Dockerfile linter with GPL-3.0 license

### Other

- Merge pull request #221 from syncable-dev/develop

## [0.26.0](https://github.com/syncable-dev/syncable-cli/compare/v0.25.0...v0.26.0) - 2025-12-21

### Added

- updated agenet behavior for better tool calling and context mngmt

## [0.25.0](https://github.com/syncable-dev/syncable-cli/compare/v0.24.5...v0.25.0) - 2025-12-20

### Added

- fixed security scan context share

### Other

- *(@ reference)* updated session logic with "@" ref

## [0.24.5](https://github.com/syncable-dev/syncable-cli/compare/v0.24.4...v0.24.5) - 2025-12-19

### Other

- Merge pull request #215 from syncable-dev/develop
- Merge pull request #145 from syncable-dev/dependabot/cargo/develop/rayon-1.11.0

## [0.24.4](https://github.com/syncable-dev/syncable-cli/compare/v0.24.3...v0.24.4) - 2025-12-19

### Added

- feat(ROADMAP updates) Updated Roadmap to reflect current progress

### Other

- bug(newline broken) Fixed a broken UI for file/folder search

## [0.24.3](https://github.com/syncable-dev/syncable-cli/compare/v0.24.2...v0.24.3) - 2025-12-19

### Added

- updated with logo

### Other

- update ROADMAP with completed features and cleaner structure

## [0.24.2](https://github.com/syncable-dev/syncable-cli/compare/v0.24.1...v0.24.2) - 2025-12-19

### Added

- *(agent)* enhance input handling with multi-line support and keyboard shortcuts

### Other

- Merge pull request #209 from syncable-dev/develop
- redesign README with AI Agent focus and improved engagement

## [0.24.1](https://github.com/syncable-dev/syncable-cli/compare/v0.24.0...v0.24.1) - 2025-12-18

### Added

- *(agent)* add @ file picker for context file selection

### Other

- Merge pull request #207 from syncable-dev/develop

## [0.24.0](https://github.com/syncable-dev/syncable-cli/compare/v0.23.1...v0.24.0) - 2025-12-18

### Added

- updated Agent dockerfile generation alongside the Syncable Cli Companion, allowing for IDE to show diff and change suggestions from the cli agent

## [0.23.1](https://github.com/syncable-dev/syncable-cli/compare/v0.23.0...v0.23.1) - 2025-12-17

### Added

- Add Syncable IDE Companion VS Code extension

### Other

- Merge branch 'main' into release-plz-2025-12-17T22-17-01Z
- release v0.23.0
- Merge pull request #202 from syncable-dev/develop

## [0.23.0](https://github.com/syncable-dev/syncable-cli/compare/v0.22.3...v0.23.0) - 2025-12-17

### Added

- Add Syncable IDE Companion VS Code extension
- VS Code extension Syncable Cli Companion

### Other

- Merge pull request #202 from syncable-dev/develop
- Merge pull request #201 from syncable-dev/develop
- Merge branch 'develop' of github.com:syncable-dev/syncable-cli into develop

## [0.22.3](https://github.com/syncable-dev/syncable-cli/compare/v0.22.2...v0.22.3) - 2025-12-17

### Other

- update Cargo.lock dependencies

## [0.22.2](https://github.com/syncable-dev/syncable-cli/compare/v0.22.1...v0.22.2) - 2025-12-17

### Added

- updated with banner, for Syncable Platform

## [0.22.1](https://github.com/syncable-dev/syncable-cli/compare/v0.22.0...v0.22.1) - 2025-12-17

### Other

- Merge pull request #193 from syncable-dev/develop

## [0.22.0](https://github.com/syncable-dev/syncable-cli/compare/v0.21.0...v0.22.0) - 2025-12-17

### Added

- Syncable Cli Agent now includes thinking and more smooth ui processing.

## [0.21.0](https://github.com/syncable-dev/syncable-cli/compare/v0.20.0...v0.21.0) - 2025-12-16

### Added

- updated agent layer, with better ui for interactivness

### Other

- Merge pull request #189 from syncable-dev/develop

## [0.20.0](https://github.com/syncable-dev/syncable-cli/compare/v0.19.0...v0.20.0) - 2025-12-16

### Added

- updated syncable-cli

### Other

- Merge pull request #187 from syncable-dev/develop

## [0.19.0](https://github.com/syncable-dev/syncable-cli/compare/v0.18.6...v0.19.0) - 2025-12-16

### Added

- Add AI agent layer with Rig framework and harden framework detection

### Other

- Merge pull request #185 from syncable-dev/develop

## [0.18.6](https://github.com/syncable-dev/syncable-cli/compare/v0.18.5...v0.18.6) - 2025-11-22

### Added

- updated framework detection

## [0.18.5](https://github.com/syncable-dev/syncable-cli/compare/v0.18.4...v0.18.5) - 2025-09-29

### Other

- update Cargo.lock dependencies

## [0.18.4](https://github.com/syncable-dev/syncable-cli/compare/v0.18.3...v0.18.4) - 2025-09-29

### Other

- update Cargo.lock dependencies

## [0.18.3](https://github.com/syncable-dev/syncable-cli/compare/v0.18.2...v0.18.3) - 2025-09-12

### Added

- Removed Update Banner on json outputs

## [0.18.2](https://github.com/syncable-dev/syncable-cli/compare/v0.18.1...v0.18.2) - 2025-09-11

### Added

- fixed vulnerability scan for js and analyzer

## [0.18.1](https://github.com/syncable-dev/syncable-cli/compare/v0.18.0...v0.18.1) - 2025-09-11

### Added

- testing analyze

## [0.18.0](https://github.com/syncable-dev/syncable-cli/compare/v0.17.0...v0.18.0) - 2025-09-11

### Added

- improved analyzer from false positives of voltagen and expo issues

### Other

- Merge pull request #159 from syncable-dev/develop

## [0.17.0](https://github.com/syncable-dev/syncable-cli/compare/v0.16.0...v0.17.0) - 2025-09-11

### Added

- test trigger
- improved telemtry and removed dublets

### Fixed

- .qodor folder for some reason wasn't corectly ignored

### Other

- added privacy-policy for telemetry
- fixed vulnerabilities output for different languages

## [0.12.1](https://github.com/syncable-dev/syncable-cli/compare/v0.12.0...v0.12.1) - 2025-07-09

### Other

- update Cargo.lock dependencies

## [0.12.0](https://github.com/syncable-dev/syncable-cli/compare/v0.11.1...v0.12.0) - 2025-07-02

### Added

- wrong named services
- test
- new cargo lock
- fixed double print

### Other

- t
- Merge branch 'main' of github.com:syncable-dev/syncable-cli into develop
- *(deps)* bump indicatif from 0.17.11 to 0.17.12
- *(deps)* bump reqwest from 0.12.20 to 0.12.21
- *(deps)* bump dashmap from 5.5.3 to 6.1.0
- *(deps)* bump rustsec from 0.30.2 to 0.30.4

## [0.11.1](https://github.com/syncable-dev/syncable-cli/compare/v0.11.0...v0.11.1) - 2025-06-20

### Added

- fixed double print

## [0.11.0](https://github.com/syncable-dev/syncable-cli/compare/v0.10.2...v0.11.0) - 2025-06-19

### Added

- feat; improved security:scaning printout
- returning dependencies as a string, for MCP server opportunity
- refactored handler logic - on to huge simplification and code breakdown

## [0.10.2](https://github.com/syncable-dev/syncable-cli/compare/v0.10.1...v0.10.2) - 2025-06-19

### Added

- returning dependencies as a string, for MCP server opportunity

## [0.10.1](https://github.com/syncable-dev/syncable-cli/compare/v0.10.0...v0.10.1) - 2025-06-19

### Added

- refactored handler logic - on to huge simplification and code breakdown

## [0.10.0](https://github.com/syncable-dev/syncable-cli/compare/v0.9.11...v0.10.0) - 2025-06-18

### Added

- refactored display

## [0.9.11](https://github.com/syncable-dev/syncable-cli/compare/v0.9.10...v0.9.11) - 2025-06-18

### Added

- added return value for handler_analyze to utilize within MCP servers
- exposing commands for lib
- added public refferences to main methods for mcp access

### Other

- Merge branch 'main' of github.com:syncable-dev/syncable-cli into develop

## [0.9.10](https://github.com/syncable-dev/syncable-cli/compare/v0.9.9...v0.9.10) - 2025-06-18

### Added

- exposing commands for lib

## [0.9.9](https://github.com/syncable-dev/syncable-cli/compare/v0.9.8...v0.9.9) - 2025-06-18

### Added

- added public refferences to main methods for mcp access
- feat added windows support
- readme updates

### Fixed

- improved security cmd, for further false postitive in terms of:

### Other

- Merge pull request #88 from syncable-dev/develop
- *(deps)* bump colored from 2.2.0 to 3.0.0 ([#87](https://github.com/syncable-dev/syncable-cli/pull/87))
- Merge branch 'main' of github.com:syncable-dev/syncable-cli into develop
- *(deps)* bump env_logger from 0.10.2 to 0.11.8
- Merge branch 'main' of github.com:syncable-dev/syncable-cli into develop
- *(deps)* bump rustsec from 0.29.3 to 0.30.2
- Merge branch 'develop' of github.com:syncable-dev/syncable-cli into develop
- *(deps)* bump clap from 4.5.39 to 4.5.40
- *(deps)* bump thiserror from 1.0.69 to 2.0.12
- *(deps)* bump proptest from 1.6.0 to 1.7.0

## [0.9.8](https://github.com/syncable-dev/syncable-cli/compare/v0.9.7...v0.9.8) - 2025-06-12

### Other

- *(deps)* bump env_logger from 0.10.2 to 0.11.8

## [0.9.7](https://github.com/syncable-dev/syncable-cli/compare/v0.9.6...v0.9.7) - 2025-06-11

### Fixed

- improved security cmd, for further false postitive in terms of:

## [0.9.6](https://github.com/syncable-dev/syncable-cli/compare/v0.9.5...v0.9.6) - 2025-06-11

### Other

- *(deps)* bump rustsec from 0.29.3 to 0.30.2

## [0.9.5](https://github.com/syncable-dev/syncable-cli/compare/v0.9.4...v0.9.5) - 2025-06-10

### Other

- update Cargo.lock dependencies

## [0.9.4](https://github.com/syncable-dev/syncable-cli/compare/v0.9.3...v0.9.4) - 2025-06-10

### Added

- feat added windows support

## [0.9.3](https://github.com/syncable-dev/syncable-cli/compare/v0.9.2...v0.9.3) - 2025-06-10

### Other

- *(deps)* bump thiserror from 1.0.69 to 2.0.12

## [0.9.2](https://github.com/syncable-dev/syncable-cli/compare/v0.9.1...v0.9.2) - 2025-06-10

### Other

- update Cargo.lock dependencies

## [0.9.1](https://github.com/syncable-dev/syncable-cli/compare/v0.9.0...v0.9.1) - 2025-06-10

### Added

- readme updates

## [0.9.0](https://github.com/syncable-dev/syncable-cli/compare/v0.8.1...v0.9.0) - 2025-06-09

### Added

- huge improvements towards security scanning and performance
- feat added python security scanning catching generat exposure secrets similar to javascript version

### Other

- Merge branch 'main' of github.com:syncable-dev/syncable-cli into develop
- README.md duplicate phrases updated

## [0.8.1](https://github.com/syncable-dev/syncable-cli/compare/v0.8.0...v0.8.1) - 2025-06-09

### Other

- Develop ([#61](https://github.com/syncable-dev/syncable-cli/pull/61))

## [0.8.0](https://github.com/syncable-dev/syncable-cli/compare/v0.7.0...v0.8.0) - 2025-06-08

### Added

- feat added python security scanning catching generat exposure secrets similar to javascript version

## [0.7.0](https://github.com/syncable-dev/syncable-cli/compare/v0.6.0...v0.7.0) - 2025-06-08

### Added

- huge improvements towards security and secret variable detection.

### Other

- updated cli-display-modes.md file for better visualization

## [0.6.0](https://github.com/syncable-dev/syncable-cli/compare/v0.5.4...v0.6.0) - 2025-06-07

### Added

- improved readme

### Fixed

- release-plz structure to avoid quick bump

### Other

- fix releaze-pls, proper section structure
- wrong release-plz setting
- small updates of unnused variables - cleanup
- updated release cycles and rules

## [0.5.4](https://github.com/syncable-dev/syncable-cli/compare/v0.5.3...v0.5.4) - 2025-06-07

### Other

- Update README.md
- Update README.md

## [0.5.3](https://github.com/syncable-dev/syncable-cli/compare/v0.5.2...v0.5.3) - 2025-06-07

### Other

- Develop ([#47](https://github.com/syncable-dev/syncable-cli/pull/47))
- Update README.md

## [0.5.2](https://github.com/syncable-dev/syncable-cli/compare/v0.5.1...v0.5.2) - 2025-06-07

### Other

- Develop ([#44](https://github.com/syncable-dev/syncable-cli/pull/44))

## [0.5.1](https://github.com/syncable-dev/syncable-cli/compare/v0.5.0...v0.5.1) - 2025-06-07

### Added

- improved README.md

## [0.5.0](https://github.com/syncable-dev/syncable-cli/compare/v0.4.2...v0.5.0) - 2025-06-06

### Other

- HOTFIX - hoping auto update becomes available

## [0.4.2](https://github.com/syncable-dev/syncable-cli/compare/v0.4.1...v0.4.2) - 2025-06-06

### Other

- Feature/improve framework and language tool detection ([#37](https://github.com/syncable-dev/syncable-cli/pull/37))

## [0.4.1](https://github.com/syncable-dev/syncable-cli/compare/v0.4.0...v0.4.1) - 2025-06-06

### Other

- Develop ([#33](https://github.com/syncable-dev/syncable-cli/pull/33))

## [0.4.0](https://github.com/syncable-dev/syncable-cli/compare/v0.3.0...v0.4.0) - 2025-06-06

### Other

- Feature/condense overview with new representation ([#29](https://github.com/syncable-dev/syncable-cli/pull/29))

## [0.3.0](https://github.com/syncable-dev/syncable-cli/compare/v0.2.1...v0.3.0) - 2025-06-06

### Added

- Added tool install verifier with cli calls ([#14](https://github.com/syncable-dev/syncable-cli/pull/14))

### Other

- Feature/extendsive docker compose and docker scan ([#25](https://github.com/syncable-dev/syncable-cli/pull/25))
- Feature/add automatic cli update ([#22](https://github.com/syncable-dev/syncable-cli/pull/22))
- Feature/update dependabot ([#11](https://github.com/syncable-dev/syncable-cli/pull/11))

## [0.2.1](https://github.com/syncable-dev/syncable-cli/compare/v0.2.0...v0.2.1) - 2025-06-06

### Other

- Feature/add automatic cli update ([#22](https://github.com/syncable-dev/syncable-cli/pull/22))

## [0.2.0](https://github.com/syncable-dev/syncable-cli/compare/v0.1.5...v0.2.0) - 2025-06-06

### Added

- Added tool install verifier with cli calls ([#14](https://github.com/syncable-dev/syncable-cli/pull/14))

## [0.1.5](https://github.com/syncable-dev/syncable-cli/compare/v0.1.4...v0.1.5) - 2025-06-06

### Added

- cargo lock update

### Other

- Feature/update dependabot ([#11](https://github.com/syncable-dev/syncable-cli/pull/11))
- Update README.md
- Update README.md
- *(deps)* bump reqwest from 0.11.27 to 0.12.19
- *(deps)* bump dirs from 5.0.1 to 6.0.0
- Feature/dependabot ([#3](https://github.com/syncable-dev/syncable-cli/pull/3))

## [0.1.4](https://github.com/syncable-dev/syncable-cli/compare/v0.1.3...v0.1.4) - 2025-06-05

### Added

- added cargo isntall command for readme
- Add new features and improvements here.

## [0.1.3] - 2024-06-05
### Added
- Initial release of `syncable-cli`.
- Analyze code repositories to detect languages, frameworks, and dependencies.
- Generate Infrastructure as Code (IaC) configurations: Dockerfile, Docker Compose, and Terraform.
- Modular architecture for extensibility and maintainability.
- CLI interface with `analyze` and `generate` commands.
- Basic security and performance analysis. 