# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.16.0](https://github.com/syncable-dev/syncable-cli/compare/v0.15.0...v0.16.0) - 2025-09-10

### Added

- open-telemtry added and improved techonology scannings

### Other

- removed telemtry for start/complete phases

## [0.15.0](https://github.com/syncable-dev/syncable-cli/compare/v0.14.0...v0.15.0) - 2025-09-10

### Added

- fixed errors
- removed warnings

### Other

- fixed vulnerabilities report

## [0.14.0](https://github.com/syncable-dev/syncable-cli/compare/v0.13.6...v0.14.0) - 2025-09-09

### Added

- added further refactor
- improved vulnerablity scanner for more that just npm audit but also bun, yarn & pnpm

### Other

- Merge branch 'main' of github.com:syncable-dev/syncable-cli into develop
- Merge branch 'develop' of github.com:syncable-dev/syncable-cli into develop

### Added
- 🧄 **Bun Runtime Integration**: Complete support for Bun JavaScript runtime and package manager
  - Automatic Bun project detection via `bun.lockb`, `bunfig.toml`, and package.json configuration
  - Multi-runtime vulnerability scanning with priority-based package manager detection (Bun > pnpm > yarn > npm)
  - Cross-platform Bun installation support (Windows PowerShell, Unix curl/bash)
  - Runtime detection with confidence levels and fallback mechanisms
  - Comprehensive unit and integration tests (34+ tests covering all scenarios)
  - Enhanced ToolDetector with caching and alternative command support
  - Updated documentation with Bun examples and migration guides

## [0.13.6](https://github.com/syncable-dev/syncable-cli/compare/v0.13.5...v0.13.6) - 2025-09-03

### Other

- update Cargo.lock dependencies

## [0.13.5](https://github.com/syncable-dev/syncable-cli/compare/v0.13.4...v0.13.5) - 2025-08-13

### Other

- update Cargo.lock dependencies

## [0.13.4](https://github.com/syncable-dev/syncable-cli/compare/v0.13.3...v0.13.4) - 2025-08-08

### Other

- update Cargo.lock dependencies

## [0.13.3](https://github.com/syncable-dev/syncable-cli/compare/v0.13.2...v0.13.3) - 2025-08-05

### Other

- update Cargo.lock dependencies

## [0.13.2](https://github.com/syncable-dev/syncable-cli/compare/v0.13.1...v0.13.2) - 2025-08-04

### Other

- update Cargo.lock dependencies

## [0.13.1](https://github.com/syncable-dev/syncable-cli/compare/v0.13.0...v0.13.1) - 2025-08-01

### Added

- updated color mode discovery

### Other

- Merge branch 'main' into develop
- *(deps)* bump toml from 0.8.23 to 0.9.3
- *(deps)* bump tokio from 1.46.1 to 1.47.0
- Merge pull request #114 from syncable-dev/dependabot/cargo/develop/tokio-1.46.1

## [0.13.0](https://github.com/syncable-dev/syncable-cli/compare/v0.12.1...v0.13.0) - 2025-07-30

### Added

- updated color mode discovery
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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