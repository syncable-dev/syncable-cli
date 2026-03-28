#!/usr/bin/env node

import { Command } from 'commander';
import inquirer from 'inquirer';
import ora from 'ora';
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

const require = createRequire(import.meta.url);
const pkg = require('../package.json');

const program = new Command();

program
  .name('syncable-cli-skills')
  .description('Install Syncable CLI skills for AI coding agents')
  .version(pkg.version);

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
        console.log(chalk.yellow(`  ⚠ sync-ctl v${syncCtlStatus.version} (outdated)`));
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
          const message = syncCtlStatus.status === 'outdated'
            ? 'Update syncable-cli via cargo?'
            : 'Install syncable-cli via cargo?';
          const { installCli } = opts.yes
            ? { installCli: true }
            : await inquirer.prompt([{ type: 'confirm', name: 'installCli', message, default: true }]);

          if (installCli) {
            const spinner = ora('  Running: cargo install syncable-cli').start();
            const force = syncCtlStatus.status === 'outdated';
            const success = await installSyncCtl(force);
            if (success) {
              spinner.succeed('  sync-ctl installed');
            } else {
              spinner.fail('  Failed to install sync-ctl. Try: cargo install syncable-cli');
            }
          }
        }
      }
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
            writeSkillsForClaude(skills, dest);
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
    console.log(`\n  Try it: Open Claude Code and say "assess this project"\n`);
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
            uninstallClaudePlugin();
            break;
          case 'codex':
            removeSyncableSkills(dest, 'syncable-*');
            break;
          case 'cursor':
            removeSyncableSkills(dest, 'syncable-*.mdc');
            break;
          case 'windsurf':
            removeSyncableSkills(dest, 'syncable-*.md');
            break;
          case 'gemini':
            removeSyncableSkills(dest, 'syncable-*');
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
    await program.commands.find((c) => c.name() === 'install')!.parseAsync(['node', 'x', '--skip-cli', ...agentsFlag, ...yesFlag, ...dryRunFlag, ...globalOnlyFlag, ...projectOnlyFlag, ...verboseFlag]);
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
    console.log();
  });

program.parse();
