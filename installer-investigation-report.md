# Syncable CLI Installer Investigation Report

**Date:** March 29, 2026
**Scope:** `npx syncable-cli-skills` installation failures across Claude Code, Gemini CLI, and Codex

---

## Executive Summary

After investigating the installer code against the official documentation for all three agents, I identified **5 critical bugs** and **3 UX problems** that explain the user-reported failures. The root causes fall into three categories:

1. **Claude Code:** The installer uses an undocumented/incorrect plugin registration method. It writes directly to internal JSON files instead of using the official CLI or settings system. Users must manually enable the plugin because the installer never actually registers it correctly.

2. **Gemini CLI:** The installer writes skills to the wrong directory (`~/.gemini/antigravity/skills/`). Gemini CLI discovers skills from `~/.gemini/skills/` (user-level) or `.gemini/skills/` (project-level), not from a profile subdirectory. The "antigravity" profile path is not a documented skill location.

3. **All agents:** `sync-ctl` installs to `~/.cargo/bin/` but agents spawn fresh shells that may not have this directory in PATH. The installer only verifies sync-ctl within its own Node.js process (where it temporarily adds cargo/bin to PATH), creating a false positive.

---

## Bug #1: Claude Code Plugin Registration is Completely Wrong

### What the installer does (WRONG)

The installer (`transformers/claude.ts`) manually writes to three internal files:

```
~/.claude/plugins/cache/syncable/syncable-cli-skills/0.1.0/skills/...
~/.claude/plugins/installed_plugins.json
~/.claude/plugins/known_marketplaces.json
```

This is the `installClaudePlugin()` function at line 143 of `transformers/claude.ts`. It creates a `plugin.json` manifest, writes skill files to a cache directory, then directly manipulates `installed_plugins.json` and `known_marketplaces.json`.

### Why this is wrong

According to the official Claude Code documentation:

- **Plugins are installed via CLI:** `claude plugin install plugin-name@marketplace-name`
- **Plugins are enabled via `enabledPlugins` in `~/.claude/settings.json`:** `{"enabledPlugins": {"syncable-cli-skills@syncable": true}}`
- **Marketplace plugins are cached to `~/.claude/plugins/cache/`** but this is an internal mechanism managed by Claude Code itself, not meant to be written to directly
- **There is no file called `installed_plugins.json`** in the documented plugin system. The installer invented this file. Plugins are tracked through `enabledPlugins` in settings.json
- **`known_marketplaces.json`** is not the documented way to register marketplaces. The correct method is either `claude plugin marketplace add` or `extraKnownMarketplaces` in `.claude/settings.json`

### Why users have to manually enable

Because the installer writes to non-standard files that Claude Code doesn't actually read for plugin activation. The plugin files exist on disk but Claude Code doesn't know they're "enabled" because `enabledPlugins` in `settings.json` was never updated.

### The fix

**Option A (Recommended): Use the CLI for programmatic installation**

```bash
# Register the marketplace
claude plugin marketplace add syncable-dev/syncable-cli

# Install the plugin
claude plugin install syncable-cli-skills@syncable --scope user
```

This is the documented, supported way. It handles caching, registration, and enabling all at once.

**Option B: Write to settings.json directly**

If you need to bypass the CLI (e.g., Claude Code isn't running), write the plugin to the cache AND update `~/.claude/settings.json`:

```json
{
  "enabledPlugins": {
    "syncable-cli-skills@syncable": true
  }
}
```

But you'd still need the marketplace registered properly. Option A is much safer.

**Option C: Use `--plugin-dir` for local plugins**

If the goal is to load a local plugin without a marketplace:

```bash
claude --plugin-dir ~/.local/share/syncable/plugin
```

This works for development/testing but isn't persistent across sessions.

---

## Bug #2: Gemini CLI Skills Go to the Wrong Directory

### What the installer does (WRONG)

The installer (`agents/gemini.ts`, line 12-38) searches for a `~/.gemini/antigravity/skills/` directory, or any profile subdirectory with a `skills/` folder:

```typescript
function findGeminiSkillsDir(): string {
  const antigravitySkills = path.join(geminiDir, 'antigravity', 'skills');
  if (fs.existsSync(antigravitySkills)) {
    return antigravitySkills;  // WRONG PATH
  }
  // Falls back to: ~/.gemini/antigravity/skills/
}
```

### Why this is wrong

According to the official Gemini CLI documentation, skills are discovered from these locations (in precedence order):

1. **Workspace skills:** `.gemini/skills/` or `.agents/skills/` (project-level)
2. **User skills:** `~/.gemini/skills/` or `~/.agents/skills/` (global)
3. **Extension skills:** bundled within installed extensions

There is **no profile subdirectory** in the skill discovery path. `~/.gemini/antigravity/` is not a documented skill location. The correct user-level path is simply `~/.gemini/skills/`.

### Why it works for some users but not others

If a user happens to have configured Gemini with custom profile settings that somehow include the "antigravity" path, it might work. But for default Gemini CLI installations, skills placed in `~/.gemini/antigravity/skills/` are invisible to Gemini because it only scans `~/.gemini/skills/`.

### The fix

Update `agents/gemini.ts`:

```typescript
export const geminiAgent: AgentConfig = {
  name: 'gemini',
  displayName: 'Gemini CLI',
  installType: 'global',
  detect: async () => {
    return fs.existsSync(path.join(os.homedir(), '.gemini'))
      || await commandExists('gemini');
  },
  getSkillPath: () => {
    // Gemini CLI discovers user skills from ~/.gemini/skills/
    // The .agents/skills/ alias also works but .gemini/skills/ is primary
    return path.join(os.homedir(), '.gemini', 'skills');
  },
};
```

Remove the entire `findGeminiSkillsDir()` function with its profile/antigravity logic.

---

## Bug #3: sync-ctl PATH Not Available to Agents

### The problem

When the installer runs `cargo install syncable-cli`, the binary goes to `~/.cargo/bin/sync-ctl`. The installer then calls `prependCargoToPath()` which does:

```typescript
export function prependCargoToPath(): void {
  process.env.PATH = `${cargoBinDir()}${path.delimiter}${process.env.PATH}`;
}
```

This only modifies the **current Node.js process** PATH. When the installer later runs `sync-ctl --version` to verify, it succeeds because the installer's own process has the modified PATH. But when an agent (Claude, Gemini, Codex) spawns a shell to run a skill command, that shell gets its PATH from the user's shell profile (`.bashrc`, `.zshrc`, `.profile`). If Rust was just installed or `~/.cargo/bin` isn't in their shell profile, `sync-ctl: command not found`.

This is exactly the bug users report: `which sync-ctl` works in their terminal (because their terminal has sourced their profile) but the agent says it's not available (because the agent's shell may have a different PATH, or the user opened a new terminal without sourcing the profile after installing Rust).

### The fix

Add a post-install verification AND a PATH setup helper:

```typescript
// After installing sync-ctl, verify it's accessible from a fresh shell
async function verifySyncCtlInPath(): Promise<boolean> {
  try {
    // Spawn a fresh login shell to check if sync-ctl is in the default PATH
    const shell = process.env.SHELL || '/bin/bash';
    await execCommand(`${shell} -l -c "which sync-ctl"`);
    return true;
  } catch {
    return false;
  }
}

// If not in PATH, offer to create a symlink in /usr/local/bin
async function ensureSyncCtlInPath(): Promise<void> {
  const inPath = await verifySyncCtlInPath();
  if (inPath) return;

  const syncCtlPath = path.join(cargoBinDir(), 'sync-ctl');
  if (!fs.existsSync(syncCtlPath)) return;

  console.log(chalk.yellow('\n  sync-ctl is installed but not in your shell PATH.'));
  console.log(chalk.yellow('  AI agents may not be able to find it.\n'));

  // Option 1: Symlink to /usr/local/bin
  const { fix } = await inquirer.prompt([{
    type: 'list',
    name: 'fix',
    message: 'How would you like to fix this?',
    choices: [
      { name: 'Create symlink in /usr/local/bin (recommended)', value: 'symlink' },
      { name: 'Add ~/.cargo/bin to shell profile', value: 'profile' },
      { name: 'Skip (I will fix it manually)', value: 'skip' },
    ],
  }]);

  if (fix === 'symlink') {
    try {
      await execCommand(`sudo ln -sf ${syncCtlPath} /usr/local/bin/sync-ctl`);
      console.log(chalk.green('  Symlink created successfully.'));
    } catch {
      console.log(chalk.red('  Failed to create symlink. Try manually:'));
      console.log(chalk.dim(`  sudo ln -sf ${syncCtlPath} /usr/local/bin/sync-ctl`));
    }
  } else if (fix === 'profile') {
    const shellProfile = getShellProfile();
    const line = 'export PATH="$HOME/.cargo/bin:$PATH"';
    try {
      fs.appendFileSync(shellProfile, `\n${line}\n`);
      console.log(chalk.green(`  Added to ${shellProfile}. Restart your terminal.`));
    } catch {
      console.log(chalk.red(`  Failed. Add this to ${shellProfile} manually:`));
      console.log(chalk.dim(`  ${line}`));
    }
  }
}
```

Additionally, **each skill's SKILL.md should include a fallback PATH** so the agent tries `~/.cargo/bin` explicitly:

```markdown
## Prerequisites
- sync-ctl binary (check with: `~/.cargo/bin/sync-ctl --version` or `sync-ctl --version`)

If sync-ctl is not found, try: `export PATH="$HOME/.cargo/bin:$PATH"` then retry.
```

---

## Bug #4: Codex Skills Require `--enable skills` Flag

### The problem

The installer places skills in `~/.agents/skills/` which is correct per the Codex documentation. However, Codex requires the user to explicitly run with `--enable skills` for skills to be active:

> "You have to run Codex with the `--enable skills` option."

The installer never tells users this. They install skills, open Codex, and the skills don't work because they're not enabled.

### The fix

Add a post-install message for Codex users:

```typescript
if (agent.name === 'codex') {
  console.log(chalk.cyan('\n  NOTE: To use skills in Codex, run:'));
  console.log(chalk.cyan('    codex --enable skills'));
  console.log(chalk.cyan('  Or invoke explicitly with: $syncable-analyze\n'));
}
```

---

## Bug #5: Gemini SKILL.md Format Missing Required Fields

### What the installer produces

The Gemini transformer (`transformers/gemini.ts`) produces:

```markdown
---
name: syncable-analyze
description: Run sync-ctl analyze for project analysis...
---

[skill body]
```

### What Gemini CLI expects

Per the documentation, Gemini CLI loads the **name and description** from frontmatter at startup and injects them into the system prompt. The model then decides whether to activate a skill. This format is actually correct for basic discovery.

However, the skill names use `syncable-` prefix which creates directory names like `syncable-analyze/SKILL.md`. This is fine structurally, but the descriptions in the skills should be more explicit about what triggers them, since Gemini only loads name+description initially and activates the full SKILL.md on demand.

### Minor fix

Ensure descriptions are optimized for Gemini's lazy-loading behavior (name + description only at startup):

```typescript
// In transformers/gemini.ts, ensure description is activation-friendly
const content = `---
name: "${skillName}"
description: "${skill.frontmatter.description}"
---

${skill.body}`;
```

---

## UX Problem #1: No Post-Install Verification

The installer shows "Setup complete!" without verifying that the skills actually work. It should:

1. Verify sync-ctl is accessible from a fresh shell
2. Verify skill files exist in the correct agent directories
3. For Claude Code: verify the plugin appears in `enabledPlugins`
4. Print agent-specific instructions (like Codex's `--enable skills`)

---

## UX Problem #2: No Error Recovery

If the installation partially fails (e.g., skills install for Claude but Gemini path is wrong), there's no way for users to diagnose what went wrong. The `status` command only counts files, it doesn't verify they're in the right place or format.

### Suggested improvement

Add a `syncable-cli-skills doctor` command that:
- Checks if sync-ctl is in PATH (from a fresh shell, not the current process)
- For each agent, verifies skills are in the documented discovery paths
- For Claude Code, checks if the plugin is actually enabled in settings.json
- For Codex, checks if `--enable skills` configuration exists

---

## UX Problem #3: Claude Plugin Requires Manual Enable

Even if the plugin registration is fixed (Bug #1), the current UX flow is:

1. User runs `npx syncable-cli-skills`
2. Installer says "Setup complete!"
3. User opens Claude Code
4. Skills don't work
5. User has to figure out they need to go to `/plugin` and enable it

### The ideal flow

1. User runs `npx syncable-cli-skills`
2. Installer detects Claude Code is installed
3. Installer runs `claude plugin install syncable-cli-skills@syncable` (which auto-enables)
4. Installer confirms: "Skills installed and enabled for Claude Code"
5. User opens Claude Code, skills work immediately

---

## Summary of Required Code Changes

| File | Bug | Change Required |
|------|-----|----------------|
| `transformers/claude.ts` | #1 | Replace `installClaudePlugin()` with `claude plugin install` CLI call or write to `enabledPlugins` in `settings.json` |
| `agents/gemini.ts` | #2 | Change `getSkillPath()` to return `~/.gemini/skills/` (remove `findGeminiSkillsDir` and all antigravity/profile logic) |
| `src/index.ts` | #3 | Add `verifySyncCtlInPath()` post-install check using a fresh login shell |
| `src/index.ts` | #3 | Add PATH fix helper (symlink or profile edit) |
| `src/index.ts` | #4 | Add Codex-specific post-install message about `--enable skills` |
| `skills/*.md` | #3 | Add `~/.cargo/bin/sync-ctl` fallback path in prerequisites |
| New: `doctor` command | UX | Add diagnostic command to verify installation health |

---

## Priority Order

1. **Bug #1 (Claude plugin)** - Highest impact, affects all Claude Code users
2. **Bug #2 (Gemini path)** - Affects all Gemini CLI users
3. **Bug #3 (PATH issue)** - Affects users who freshly install Rust
4. **Bug #4 (Codex enable)** - Simple fix, just needs a message
5. **UX improvements** - Important but not blocking
