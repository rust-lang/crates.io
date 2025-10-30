#!/usr/bin/env node

import { execSync } from 'node:child_process';

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
  const output = execSync(`git log --format="${format}" ${from}..${to}`, {
    encoding: 'utf8',
    stdio: ['pipe', 'pipe', 'pipe'],
  });

  return output
    .trim()
    .split('\n')
    .filter(Boolean)
    .map(line => {
      const [sha, author, message] = line.split('\u0000');
      return { sha, author, message };
    });
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
  } catch (error) {
    console.error(`Error: ${error.message}`);
    process.exit(1);
  }
}

main();
