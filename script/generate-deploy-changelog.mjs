#!/usr/bin/env node

import { execSync } from 'node:child_process';

function exec(command) {
  return execSync(command, {
    encoding: 'utf8',
    stdio: ['pipe', 'pipe', 'pipe'],
  });
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

    console.log(`Found ${commits.length} commit${commits.length === 1 ? '' : 's'}:\n`);

    for (const commit of commits) {
      console.log(`${commit.sha.slice(0, 8)}\t${commit.author}\t${commit.message}`);
    }

    const migrations = getMigrations(from, to);

    if (migrations.length !== 0) {
      console.log(`\n${'='.repeat(80)}`);
      console.log(`Database Migrations (${migrations.length}):`);
      console.log('='.repeat(80));

      for (const migration of migrations) {
        console.log(`\n${migration.dir}`);
        console.log('-'.repeat(80));

        if (migration.upContent) {
          console.log('\nup.sql:');
          console.log(migration.upContent);
        }

        if (migration.downContent) {
          console.log('down.sql:');
          console.log(migration.downContent);
        }
      }
    }
  } catch (error) {
    console.error(`Error: ${error.message}`);
    process.exit(1);
  }
}

main();
