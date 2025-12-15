#!/usr/bin/env node
import { execSync } from 'node:child_process';

function exec(command, options = {}) {
  return execSync(command, {
    encoding: 'utf8',
    stdio: ['pipe', 'pipe', 'pipe'],
    ...options,
  });
}

function isClaudeAvailable() {
  try {
    exec('which claude');
    return true;
  } catch {
    return false;
  }
}

function parseGitHubCompareUrl(url) {
  const compareMatch = url.match(/compare\/([a-f0-9]+)\.\.\.([a-f0-9]+)/);
  if (!compareMatch) {
    throw new Error('Invalid GitHub compare URL. Expected format: https://github.com/owner/repo/compare/hash1...hash2');
  }
  return {
    from: compareMatch[1],
    to: compareMatch[2],
  };
}

function getCommits(from, to) {
  const format = '%H%x00%an%x00%s';
  const output = exec(`git log --format="${format}" ${from}..${to}`);

  return output
    .trim()
    .split('\n')
    .filter(Boolean)
    .map(line => {
      const [sha, author, message] = line.split('\u0000');
      return { sha, author, message };
    });
}

function getMigrations(from, to) {
  try {
    const files = exec(`git log --name-only --format="" ${from}..${to} -- migrations/`);

    const migrationFiles = files
      .trim()
      .split('\n')
      .filter(Boolean)
      .filter(file => file.endsWith('.sql'));

    if (migrationFiles.length === 0) {
      return [];
    }

    const migrations = [];
    const migrationDirs = new Set(migrationFiles.map(file => file.split('/').slice(0, 2).join('/')));

    for (const dir of migrationDirs) {
      const upFile = `${dir}/up.sql`;
      const downFile = `${dir}/down.sql`;

      let upContent = '';
      let downContent = '';

      if (migrationFiles.includes(upFile)) {
        try {
          upContent = exec(`git show ${to}:${upFile}`);
        } catch (error) {
          console.error(`Warning: Could not read ${upFile}: ${error.message}`);
        }
      }

      if (migrationFiles.includes(downFile)) {
        try {
          downContent = exec(`git show ${to}:${downFile}`);
        } catch (error) {
          console.error(`Warning: Could not read ${downFile}: ${error.message}`);
        }
      }

      migrations.push({
        dir,
        upContent,
        downContent,
      });
    }

    return migrations;
  } catch (error) {
    console.error(`Warning: Could not retrieve migrations: ${error.message}`);
    return [];
  }
}

function formatCommits(commits) {
  return commits.map(c => `${c.sha.slice(0, 8)}\t${c.author}\t${c.message}`).join('\n');
}

function formatMigrations(migrations) {
  let output = `\n${'='.repeat(80)}\n`;
  output += `Database Migrations (${migrations.length}):\n`;
  output += '='.repeat(80);

  if (migrations.length === 0) {
    output += `\nNo database migrations`;
  }

  for (const migration of migrations) {
    output += `\n\n${migration.dir}\n`;
    output += '-'.repeat(80);

    if (migration.upContent) {
      output += '\n\nup.sql:\n';
      output += migration.upContent;
    }

    if (migration.downContent) {
      output += '\ndown.sql:\n';
      output += migration.downContent;
    }
  }

  return output;
}

function generateChangelog(commits, migrations, url) {
  const commitList = formatCommits(commits);
  const migrationInfo = formatMigrations(migrations);

  const prompt = `You are generating a deployment changelog for crates.io based on git commit history and database migrations.

Generate a deployment announcement in this exact style:

:rocket:  **Deploying to production**

This deployment:
- [bullet point summarizing a key change]
- [bullet point summarizing another key change]
- updates dependencies

[Statement about database migrations and rollback safety]

${url}

Guidelines:
- Start with ":rocket:  **Deploying to production**" (note: two spaces after emoji)
- Use "This deployment:" followed by a bulleted list
- Group related commits into single bullets
- Always include "updates dependencies" as the last bullet if there are dependency updates (Renovate commits)
- Focus on user-facing changes and significant internal changes
- For the migration statement:
  - If no migrations: "It contains no database migrations and should be safe to rollback."
  - If migrations that are safe: "**It contains [describe migration], but should be safe to rollback.**"
  - If migrations need care: "**This deployment contains [describe migration]. [Explain rollback implications]**"
- End with the GitHub compare URL
- Be concise but informative
- Use present tense

Commit history:
${commitList}
${migrationInfo}

Generate only the deployment announcement, no additional explanation.`;

  try {
    return exec('claude', { input: prompt }).trim();
  } catch (error) {
    throw new Error(`Failed to generate changelog with Claude CLI: ${error.message}`);
  }
}

function main() {
  const url = process.argv[2];

  if (!url) {
    console.error('Usage: script/generate-deploy-changelog.mjs <github-compare-url>');
    console.error(
      'Example: script/generate-deploy-changelog.mjs https://github.com/rust-lang/crates.io/compare/f4990229...3c279fb5',
    );
    process.exit(1);
  }

  try {
    const { from, to } = parseGitHubCompareUrl(url);
    const commits = getCommits(from, to);

    if (commits.length === 0) {
      console.log('No commits found in range');
      return;
    }

    const migrations = getMigrations(from, to);

    console.log(`Found ${commits.length} commit${commits.length === 1 ? '' : 's'}:\n`);
    console.log(formatCommits(commits));
    console.log(formatMigrations(migrations));

    if (isClaudeAvailable()) {
      console.log(`\n${'='.repeat(80)}`);
      console.log('Generating deployment changelog...\n');
      const changelog = generateChangelog(commits, migrations, url);
      console.log(changelog);
    }
  } catch (error) {
    console.error(`Error: ${error.message}`);
    process.exit(1);
  }
}

main();
