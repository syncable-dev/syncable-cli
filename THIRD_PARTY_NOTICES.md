# Third Party Notices

This file contains attributions and license information for third-party software
incorporated into Syncable-CLI.

---

## Hadolint

The Dockerfile linting functionality in `src/analyzer/hadolint/` is a Rust
translation of the original Hadolint project.

**Original Project:** [Hadolint](https://github.com/hadolint/hadolint)

**Original Authors:**
- Lukas Martinelli (lukasmartinelli)
- Lorenzo Bolla (lbolla)
- And all contributors to the Hadolint project

**Original License:** GNU General Public License v3.0 (GPL-3.0)

**Original Copyright:**
```
Copyright (c) 2016-2024 Lukas Martinelli and contributors
```

**What was translated:**
- Dockerfile parsing logic (originally in Haskell)
- Lint rule definitions (DL3xxx, DL4xxx series)
- Pragma/ignore directive handling
- Configuration file format
- Rule severity and messaging

**Modifications made:**
- Complete rewrite from Haskell to Rust
- Integration with Syncable-CLI's agent and tool system
- Native async support for streaming output
- Adaptation to Rust error handling patterns
- Additional rules and improvements specific to Syncable's use cases

**License Notice:**
This derivative work is licensed under GPL-3.0, as required by the original
Hadolint license. See the LICENSE file in the root of this repository.

The full text of the GPL-3.0 license can be found at:
https://www.gnu.org/licenses/gpl-3.0.en.html

---

## Docker Compose Linter

The Docker Compose linting functionality in `src/analyzer/dclint/` is a Rust
implementation inspired by the docker-compose-linter project.

**Original Project:** [docker-compose-linter](https://github.com/zavoloklom/docker-compose-linter)

**Original Author:** Sergey Suspended (zavoloklom)

**Original License:** MIT License

**Original Copyright:**
```
Copyright (c) 2024 Sergey Suspended
```

**What was implemented:**
- Docker Compose YAML validation logic
- Lint rule concepts (DCL001-DCL015 series)
- Service configuration validation patterns
- Best practices enforcement

**Modifications made:**
- Complete implementation in Rust (original was TypeScript)
- Integration with Syncable-CLI's agent and tool system
- Native async support for streaming output
- Adaptation to Rust error handling patterns
- Additional rules and improvements specific to Syncable's use cases

**License Notice:**
The original docker-compose-linter is licensed under MIT. Our Rust implementation
is original code inspired by the rule concepts and validation patterns from the
original project.

---

## ShellCheck (Rule Concepts)

Some shell-related lint rules are inspired by ShellCheck.

**Original Project:** [ShellCheck](https://github.com/koalaman/shellcheck)

**Original Author:** Vidar Holen (koalaman)

**Original License:** GNU General Public License v3.0 (GPL-3.0)

**Note:** Syncable-CLI does not include ShellCheck code directly. The shell
analysis rules are original implementations inspired by ShellCheck's rule
concepts and documentation.

---

## Acknowledgments

We are grateful to the open source community and the authors of Hadolint and
docker-compose-linter for creating and maintaining excellent container configuration
linting tools. These Rust implementations allow native integration with Syncable-CLI
while preserving the valuable rule definitions and linting logic developed by the
original authors.

If you are the author of any software mentioned here and believe the attribution
is incorrect or incomplete, please open an issue at:
https://github.com/syncable-dev/syncable-cli/issues
