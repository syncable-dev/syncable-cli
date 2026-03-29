#!/usr/bin/env node

import { Command } from 'commander';
import inquirer from 'inquirer';
import ora from 'ora';
import path from 'path';
import os from 'os';
import fs from 'fs';
import chalk from 'chalk';
import { createRequire } from 'module';
import { checkNodeVersion, checkCargo, checkSyncCtl } from './prerequisites/check.js';
import { installRustup } from './prerequisites/install-rustup.js';
import { installSyncCtl } from './prerequisites/install-cli.js';
import { detectAgents, allAgents } from './agents/detect.js';
import { AgentConfig, AgentName } from './agents/types.js';
import { loadSkills, getBundledSkillsDir } from './skills.js';
import {
  writeSkillsForClaude,
  writeSkillsForCodex,
  writeSkillsForCursor,
  writeSkillsForWindsurf,
  writeSkillsForGemini,
  InstallOptions,
} from './commands/install.js';
import { removeSyncableSkills, removeGeminiSection } from './commands/uninstall.js';
import { uninstallClaudePlugin } from './transformers/claude.js';
import { countInstalledSkills } from './commands/status.js';
import { isSyncCtlInLoginShell, getShellProfile, cargoBinDir, execCommand, isWindows } from './utils.js';

const require = createRequire(import.meta.url);
const pkg = require('../package.json');

const program = new Command();

program
  .name('syncable-cli-skills')
  .description('Install Syncable CLI skills for AI coding agents')
  .version(pkg.version);

/**
 * Verify sync-ctl is accessible from a fresh login shell and fix PATH if needed.
 * This ensures AI agents (which spawn fresh shells) can actually find sync-ctl.
 */
async function verifySyncCtlPath(opts: { yes?: boolean }): Promise<void> {
  const inLoginShell = await isSyncCtlInLoginShell();
  if (inLoginShell) {
    console.log(chalk.green('  ✓ sync-ctl accessible from shell PATH'));
    return;
  }

  const syncCtlBinary = path.join(cargoBinDir(), isWindows() ? 'sync-ctl.exe' : 'sync-ctl');
  if (!fs.existsSync(syncCtlBinary)) {
    // Binary doesn't exist at all — nothing to fix here
    return;
  }

  console.log(chalk.yellow('\n  ⚠ sync-ctl is installed but NOT in your shell PATH.'));
  console.log(chalk.yellow('    AI agents will fail with "sync-ctl: command not found".\n'));

  if (isWindows()) {
    console.log(chalk.cyan('  To fix, add this to your system PATH:'));
    console.log(chalk.dim(`    ${cargoBinDir()}\n`));
    return;
  }

  const choices = [
    { name: 'Create symlink in /usr/local/bin (recommended, may need sudo)', value: 'symlink' },
    { name: `Add ~/.cargo/bin to shell profile (${getShellProfile()})`, value: 'profile' },
    { name: 'Skip — I will fix it manually', value: 'skip' },
  ];

  const { fix } = opts.yes
    ? { fix: 'profile' }
    : await inquirer.prompt([{
        type: 'list',
        name: 'fix',
        message: 'How would you like to fix this?',
        choices,
      }]);

  if (fix === 'symlink') {
    const spinner = ora('  Creating symlink...').start();
    try {
      await execCommand(`sudo ln -sf "${syncCtlBinary}" /usr/local/bin/sync-ctl`);
      spinner.succeed('  Symlink created: /usr/local/bin/sync-ctl');
    } catch {
      spinner.fail('  Failed to create symlink (sudo may have been denied)');
      console.log(chalk.dim(`    Try manually: sudo ln -sf "${syncCtlBinary}" /usr/local/bin/sync-ctl`));
    }
  } else if (fix === 'profile') {
    const profilePath = getShellProfile();
    const exportLine = 'export PATH="$HOME/.cargo/bin:$PATH"';

    // Check if it's already in the profile
    try {
      const profileContent = fs.existsSync(profilePath) ? fs.readFileSync(profilePath, 'utf-8') : '';
      if (profileContent.includes('.cargo/bin')) {
        console.log(chalk.yellow(`  ~/.cargo/bin is already in ${profilePath} but your current shell hasn't sourced it.`));
        console.log(chalk.cyan(`  Run: source ${profilePath}\n`));
        return;
      }
    } catch {
      // Can't read profile — proceed with append
    }

    try {
      fs.appendFileSync(profilePath, `\n# Added by syncable-cli-skills installer\n${exportLine}\n`);
      console.log(chalk.green(`  ✓ Added to ${profilePath}`));
      console.log(chalk.cyan(`    Restart your terminal or run: source ${profilePath}\n`));
    } catch {
      console.log(chalk.red(`  Failed to update ${profilePath}. Add this line manually:`));
      console.log(chalk.dim(`    ${exportLine}\n`));
    }
  } else {
    console.log(chalk.dim(`  To fix later, run: export PATH="$HOME/.cargo/bin:$PATH"\n`));
  }
}

/**
 * Print agent-specific post-install instructions that users need to know.
 */
function printPostInstallNotes(agents: AgentConfig[]): void {
  const notes: string[] = [];

  for (const agent of agents) {
    switch (agent.name) {
      case 'claude':
        notes.push(
          `  ${chalk.cyan('Claude Code')}: Skills are auto-enabled. If they don't appear:`,
          `    1. Run ${chalk.bold('/reload-plugins')} inside Claude Code`,
          `    2. Or manually: ${chalk.bold('/plugin marketplace add syncable-dev/syncable-cli')}`,
          `       then: ${chalk.bold('/plugin install syncable-cli-skills@syncable')}`,
        );
        break;

      case 'codex':
        notes.push(
          `  ${chalk.cyan('Codex')}: You must enable skills when starting Codex:`,
          `    ${chalk.bold('codex --enable skills')}`,
          `    Or invoke explicitly: ${chalk.bold('$syncable-analyze')}`,
          `    Skills installed to: ${chalk.dim('~/.codex/skills/')}`,
        );
        break;

      case 'gemini':
        notes.push(
          `  ${chalk.cyan('Gemini CLI')}: Skills are auto-discovered. Verify with:`,
          `    ${chalk.bold('/skills list')} inside Gemini CLI`,
          `    Skills installed to: ${chalk.dim('~/.gemini/skills/')}`,
        );
        break;

      case 'cursor':
        notes.push(
          `  ${chalk.cyan('Cursor')}: Rules are loaded automatically in projects.`,
        );
        break;

      case 'windsurf':
        notes.push(
          `  ${chalk.cyan('Windsurf')}: Rules are loaded automatically in projects.`,
        );
        break;
    }
  }

  if (notes.length > 0) {
    console.log(chalk.bold('\n  Agent-specific notes:\n'));
    for (const note of notes) {
      console.log(note);
    }
    console.log();
  }
}

program
  .command('install', { isDefault: true })
  .description('Install sync-ctl and skills')
  .option('--skip-cli', 'Skip sync-ctl installation check')
  .option('--dry-run', 'Show what would be done without doing it')
  .option('--agents <list>', 'Comma-separated agent list')
  .option('--global-only', 'Only install global skills')
  .option('--project-only', 'Only install project-level rules')
  .option('-y, --yes', 'Skip confirmations')
  .option('--verbose', 'Show detailed output')
  .action(async (opts) => {
    const verbose = opts.verbose || false;
    console.log(chalk.bold('\n  Syncable CLI Skills Installer'));
    console.log('  ' + '─'.repeat(29) + '\n');

    // Check Node.js version
    const nodeCheck = checkNodeVersion();
    if (nodeCheck.status === 'outdated') {
      console.error(chalk.red(`  Node.js >= 18.0.0 required. Found: ${nodeCheck.version}`));
      process.exit(1);
    }
    console.log(chalk.green(`  ✓ Node.js ${nodeCheck.version}`));

    // Check prerequisites
    if (!opts.skipCli) {
      const cargoStatus = await checkCargo();
      const syncCtlStatus = await checkSyncCtl();

      if (cargoStatus.status === 'ok') {
        console.log(chalk.green(`  ✓ cargo ${cargoStatus.version}`));
      } else {
        console.log(chalk.red('  ✗ cargo not found'));
      }

      if (syncCtlStatus.status === 'ok') {
        console.log(chalk.green(`  ✓ sync-ctl v${syncCtlStatus.version}`));
      } else if (syncCtlStatus.status === 'outdated') {
        const latestInfo = syncCtlStatus.latestVersion ? ` → ${syncCtlStatus.latestVersion} available` : '';
        console.log(chalk.yellow(`  ⚠ sync-ctl v${syncCtlStatus.version} (outdated${latestInfo})`));
      } else {
        console.log(chalk.red('  ✗ sync-ctl not found'));
      }

      // Install missing prerequisites
      if (cargoStatus.status === 'missing') {
        console.log(chalk.yellow('\n  sync-ctl requires Rust\'s cargo package manager.\n'));
        const { installRust } = opts.yes
          ? { installRust: true }
          : await inquirer.prompt([{ type: 'confirm', name: 'installRust', message: 'Install Rust toolchain via rustup?', default: true }]);

        if (installRust) {
          const spinner = ora('  Installing rustup...').start();
          const success = await installRustup();
          if (success) {
            spinner.succeed('  Rust toolchain installed');
          } else {
            spinner.fail('  Failed to install Rust. Install manually: https://rustup.rs');
          }
        }
      }

      if (syncCtlStatus.status === 'missing' || syncCtlStatus.status === 'outdated') {
        const cargoNow = await checkCargo();
        if (cargoNow.status === 'ok') {
          if (syncCtlStatus.status === 'outdated') {
            // Always auto-upgrade to latest — no prompt needed
            const latestLabel = syncCtlStatus.latestVersion ? ` to v${syncCtlStatus.latestVersion}` : '';
            const spinner = ora(`  Upgrading sync-ctl${latestLabel}...`).start();
            const success = await installSyncCtl(true); // force = true for upgrade
            if (success) {
              spinner.succeed(`  sync-ctl upgraded${latestLabel}`);
            } else {
              spinner.fail('  Failed to upgrade sync-ctl. Try: cargo install syncable-cli --force');
            }
          } else {
            // Missing — ask to install
            const { installCli } = opts.yes
              ? { installCli: true }
              : await inquirer.prompt([{ type: 'confirm', name: 'installCli', message: 'Install syncable-cli via cargo?', default: true }]);

            if (installCli) {
              const spinner = ora('  Running: cargo install syncable-cli').start();
              const success = await installSyncCtl(false);
              if (success) {
                spinner.succeed('  sync-ctl installed');
              } else {
                spinner.fail('  Failed to install sync-ctl. Try: cargo install syncable-cli');
              }
            }
          }
        }
      }

      // Verify sync-ctl is actually in the shell PATH (not just this process)
      await verifySyncCtlPath(opts);
    }

    // Detect agents
    console.log(chalk.bold('\n  Detecting AI coding agents...\n'));
    const detectionResults = await detectAgents();

    for (const { agent, detected } of detectionResults) {
      if (detected) {
        console.log(chalk.green(`  ✓ ${agent.displayName} detected`));
        if (verbose) console.log(chalk.dim(`    path: ${agent.getSkillPath()}`));
      } else {
        console.log(chalk.dim(`  ✗ ${agent.displayName} not detected`));
      }
    }

    // Determine which agents to install for
    let selectedAgents: AgentConfig[];

    if (opts.agents) {
      const names = opts.agents.split(',').map((n: string) => n.trim()) as AgentName[];
      selectedAgents = allAgents().filter((a) => names.includes(a.name));
    } else if (opts.globalOnly) {
      selectedAgents = detectionResults.filter((r) => r.detected && r.agent.installType === 'global').map((r) => r.agent);
    } else if (opts.projectOnly) {
      selectedAgents = detectionResults.filter((r) => r.detected && r.agent.installType === 'project').map((r) => r.agent);
    } else if (opts.yes) {
      selectedAgents = detectionResults.filter((r) => r.detected).map((r) => r.agent);
    } else {
      const choices = detectionResults.map((r) => ({
        name: `${r.agent.displayName} — ${r.agent.installType} install`,
        value: r.agent.name,
        checked: r.detected,
      }));

      const { agents } = await inquirer.prompt([{
        type: 'checkbox',
        name: 'agents',
        message: 'Which agents should receive Syncable skills?',
        choices,
      }]);

      selectedAgents = allAgents().filter((a) => agents.includes(a.name));
    }

    if (selectedAgents.length === 0) {
      if (opts.globalOnly) {
        console.log(chalk.yellow('\n  No global agents detected (Claude Code, Codex). Nothing to install.'));
      } else if (opts.projectOnly) {
        console.log(chalk.yellow('\n  No project agents detected (Cursor, Windsurf, Gemini). Nothing to install.'));
      } else {
        console.log(chalk.yellow('\n  No agents selected. Nothing to install.'));
      }
      return;
    }

    // Load and install skills
    const skills = loadSkills(getBundledSkillsDir());
    const commandCount = skills.filter((s) => s.category === 'command').length;
    const workflowCount = skills.filter((s) => s.category === 'workflow').length;

    for (const agent of selectedAgents) {
      const spinner = ora(`  Installing skills for ${agent.displayName}...`).start();

      if (opts.dryRun) {
        spinner.info(`  Would install ${skills.length} skills for ${agent.displayName}`);
        continue;
      }

      try {
        const dest = agent.getSkillPath();
        switch (agent.name) {
          case 'claude':
            await writeSkillsForClaude(skills, dest);
            break;
          case 'codex':
            writeSkillsForCodex(skills, dest);
            break;
          case 'cursor':
            writeSkillsForCursor(skills, dest);
            break;
          case 'windsurf':
            writeSkillsForWindsurf(skills, dest);
            break;
          case 'gemini':
            writeSkillsForGemini(skills, dest);
            break;
        }
        spinner.succeed(`  ${skills.length} skills installed for ${agent.displayName}`);
        if (verbose) console.log(chalk.dim(`    dest: ${dest}`));
      } catch (err) {
        spinner.fail(`  Failed to install skills for ${agent.displayName}: ${err}`);
      }
    }

    // Summary
    console.log('\n  ' + '─'.repeat(29));
    console.log(chalk.green.bold('  ✓ Setup complete!\n'));
    console.log(`  Installed:`);
    console.log(`    • ${commandCount} command skills + ${workflowCount} workflow skills`);
    console.log(`    • Agents: ${selectedAgents.map((a) => a.displayName).join(', ')}`);

    // Print agent-specific post-install notes (Codex --enable skills, etc.)
    printPostInstallNotes(selectedAgents);

    // Manual install fallback — if installer didn't work for some reason
    console.log(chalk.dim('  If skills are not loading, install manually from GitHub:'));
    console.log(chalk.dim('    https://github.com/syncable-dev/syncable-cli/tree/main/installer/skills'));
    console.log();
    console.log(`  Try it: Open your agent and say "assess this project"\n`);
  });

program
  .command('uninstall')
  .description('Remove skills from agents')
  .option('--agents <list>', 'Comma-separated agent list')
  .option('-y, --yes', 'Skip confirmations')
  .action(async (opts) => {
    const agents = opts.agents
      ? allAgents().filter((a) => opts.agents.split(',').includes(a.name))
      : allAgents();

    if (!opts.yes) {
      const { confirm } = await inquirer.prompt([{
        type: 'confirm',
        name: 'confirm',
        message: `Remove Syncable skills from ${agents.map((a) => a.displayName).join(', ')}?`,
        default: false,
      }]);
      if (!confirm) return;
    }

    for (const agent of agents) {
      const spinner = ora(`  Removing skills from ${agent.displayName}...`).start();
      try {
        const dest = agent.getSkillPath();
        switch (agent.name) {
          case 'claude':
            await uninstallClaudePlugin();
            break;
          case 'codex':
            removeSyncableSkills(dest, 'syncable-*');
            // Also clean old location (~/.codex/skills/)
            removeSyncableSkills(path.join(os.homedir(), '.codex', 'skills'), 'syncable-*');
            break;
          case 'cursor':
            removeSyncableSkills(dest, 'syncable-*.mdc');
            break;
          case 'windsurf':
            removeSyncableSkills(dest, 'syncable-*.md');
            break;
          case 'gemini':
            removeSyncableSkills(dest, 'syncable-*');
            // Also clean old antigravity profile location from previous installer versions
            const oldGeminiDir = path.join(os.homedir(), '.gemini', 'antigravity', 'skills');
            if (fs.existsSync(oldGeminiDir)) {
              removeSyncableSkills(oldGeminiDir, 'syncable-*');
            }
            break;
        }
        spinner.succeed(`  Skills removed from ${agent.displayName}`);
      } catch (err) {
        spinner.fail(`  Failed to remove skills from ${agent.displayName}: ${err}`);
      }
    }
  });

program
  .command('update')
  .description('Update skills to latest version')
  .option('--agents <list>', 'Comma-separated agent list')
  .option('--dry-run', 'Show what would be done without doing it')
  .option('--global-only', 'Only update global skills')
  .option('--project-only', 'Only update project-level rules')
  .option('-y, --yes', 'Skip confirmations')
  .option('--verbose', 'Show detailed output')
  .action(async (opts) => {
    const yesFlag = opts.yes ? ['--yes'] : [];
    const agentsFlag = opts.agents ? ['--agents', opts.agents] : [];
    const dryRunFlag = opts.dryRun ? ['--dry-run'] : [];
    const globalOnlyFlag = opts.globalOnly ? ['--global-only'] : [];
    const projectOnlyFlag = opts.projectOnly ? ['--project-only'] : [];
    const verboseFlag = opts.verbose ? ['--verbose'] : [];
    await program.commands.find((c) => c.name() === 'uninstall')!.parseAsync(['node', 'x', ...agentsFlag, ...yesFlag]);
    // NOTE: Do NOT pass --skip-cli here — update must always check for and install the latest sync-ctl
    await program.commands.find((c) => c.name() === 'install')!.parseAsync(['node', 'x', ...agentsFlag, ...yesFlag, ...dryRunFlag, ...globalOnlyFlag, ...projectOnlyFlag, ...verboseFlag]);
  });

program
  .command('status')
  .description('Show what is installed and where')
  .action(async () => {
    console.log(chalk.bold('\n  Syncable CLI Skills Status\n'));

    const detectionResults = await detectAgents();
    const syncCtlStatus = await checkSyncCtl();
    const cargoStatus = await checkCargo();

    console.log('  Agent         Status       Location');
    console.log('  ' + '─'.repeat(60));

    for (const { agent } of detectionResults) {
      const dest = agent.getSkillPath();
      const count = countInstalledSkills(dest, agent.name);
      if (count > 0) {
        console.log(`  ${agent.displayName.padEnd(14)} ${chalk.green('✓ installed')}  ${dest} (${count} skills)`);
      } else {
        console.log(`  ${agent.displayName.padEnd(14)} ${chalk.dim('✗ not installed')}`);
      }
    }

    console.log();
    if (syncCtlStatus.status === 'ok') {
      console.log(`  sync-ctl      ${chalk.green('✓')} v${syncCtlStatus.version}`);
    } else {
      console.log(`  sync-ctl      ${chalk.red('✗ not found')}`);
    }
    if (cargoStatus.status === 'ok') {
      console.log(`  cargo         ${chalk.green('✓')} v${cargoStatus.version}`);
    } else {
      console.log(`  cargo         ${chalk.red('✗ not found')}`);
    }

    // Check if sync-ctl is visible in login shell
    const inPath = await isSyncCtlInLoginShell();
    if (syncCtlStatus.status === 'ok' && !inPath) {
      console.log(chalk.yellow(`\n  ⚠ sync-ctl is installed but NOT in your shell PATH.`));
      console.log(chalk.yellow(`    AI agents may not be able to run skills.`));
      console.log(chalk.dim(`    Fix: run "syncable-cli-skills install" to update your PATH`));
    }

    console.log();
  });

program
  .command('doctor')
  .description('Diagnose installation health')
  .action(async () => {
    console.log(chalk.bold('\n  Syncable CLI Skills Doctor\n'));
    let issues = 0;

    // 1. Check sync-ctl binary exists
    const syncCtlStatus = await checkSyncCtl();
    if (syncCtlStatus.status === 'ok') {
      console.log(chalk.green(`  ✓ sync-ctl v${syncCtlStatus.version} installed`));
    } else {
      console.log(chalk.red('  ✗ sync-ctl not installed'));
      console.log(chalk.dim('    Fix: cargo install syncable-cli'));
      issues++;
    }

    // 2. Check sync-ctl is in login shell PATH
    const inPath = await isSyncCtlInLoginShell();
    if (inPath) {
      console.log(chalk.green('  ✓ sync-ctl accessible from shell PATH'));
    } else if (syncCtlStatus.status === 'ok') {
      console.log(chalk.red('  ✗ sync-ctl NOT in shell PATH — agents will fail'));
      console.log(chalk.dim('    Fix: run "syncable-cli-skills install" to update PATH'));
      issues++;
    }

    // 3. Check each agent's skill directory
    const detectionResults = await detectAgents();
    for (const { agent, detected } of detectionResults) {
      if (!detected) continue;

      const dest = agent.getSkillPath();
      const count = countInstalledSkills(dest, agent.name);
      if (count > 0) {
        console.log(chalk.green(`  ✓ ${agent.displayName}: ${count} skills at ${dest}`));
      } else {
        console.log(chalk.red(`  ✗ ${agent.displayName}: no skills found at ${dest}`));
        issues++;
      }

      // Claude-specific: check enabledPlugins
      if (agent.name === 'claude') {
        const settingsFile = path.join(os.homedir(), '.claude', 'settings.json');
        try {
          const settings = JSON.parse(fs.readFileSync(settingsFile, 'utf-8'));
          const key = 'syncable-cli-skills@syncable';
          if (settings.enabledPlugins && settings.enabledPlugins[key] === true) {
            console.log(chalk.green('  ✓ Claude Code: plugin enabled in settings.json'));
          } else {
            console.log(chalk.red('  ✗ Claude Code: plugin NOT enabled in settings.json'));
            console.log(chalk.dim('    Fix: run "syncable-cli-skills install --agents claude"'));
            issues++;
          }
        } catch {
          console.log(chalk.red('  ✗ Claude Code: could not read settings.json'));
          issues++;
        }
      }
    }

    console.log('\n  ' + '─'.repeat(40));
    if (issues === 0) {
      console.log(chalk.green.bold('  ✓ Everything looks good!\n'));
    } else {
      console.log(chalk.yellow.bold(`  Found ${issues} issue${issues === 1 ? '' : 's'} — see above for fixes.\n`));
    }
  });

program.parse();
