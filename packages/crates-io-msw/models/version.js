import { nullable, oneOf, primaryKey } from '@mswjs/data';

import { applyDefault } from '../utils/defaults.js';

const LICENSES = ['MIT/Apache-2.0', 'MIT', 'Apache-2.0'];

const LANGUAGES = ['Rust', 'JavaScript', 'TypeScript', 'Python', 'CSS', 'HTML', 'Shell'];

export default {
  id: primaryKey(Number),

  num: String,
  created_at: String,
  updated_at: String,
  yanked: Boolean,
  yank_message: nullable(String),
  license: String,
  downloads: Number,
  features: Object,
  crate_size: Number,
  readme: nullable(String),
  rust_version: nullable(String),
  trustpub_data: nullable(Object),
  linecounts: nullable(Object),

  crate: oneOf('crate'),
  publishedBy: nullable(oneOf('user')),

  preCreate(attrs, counter) {
    applyDefault(attrs, 'id', () => counter);
    applyDefault(attrs, 'num', () => `1.0.${attrs.id - 1}`);
    applyDefault(attrs, 'created_at', () => '2010-06-16T21:30:45Z');
    applyDefault(attrs, 'updated_at', () => '2017-02-24T12:34:56Z');
    applyDefault(attrs, 'yanked', () => false);
    applyDefault(attrs, 'yank_message', () => null);
    applyDefault(attrs, 'license', () => LICENSES[attrs.id % LICENSES.length]);
    applyDefault(attrs, 'downloads', () => (((attrs.id + 13) * 42) % 13) * 1234);
    applyDefault(attrs, 'crate_size', () => (((attrs.id + 13) * 42) % 13) * 54_321);
    applyDefault(attrs, 'readme', () => null);
    applyDefault(attrs, 'rust_version', () => null);
    applyDefault(attrs, 'trustpub_data', () => null);
    applyDefault(attrs, 'linecounts', () => generateLinecounts(attrs.id));

    if (!attrs.crate) {
      throw new Error(`Missing \`crate\` relationship on \`version:${attrs.num}\``);
    }
  },
};

function generateLinecounts(id) {
  // Some versions don't have linecount data (simulating older versions)
  if (id % 4 === 0) {
    return null;
  }

  const languages = {};
  let totalCodeLines = 0;
  let totalCommentLines = 0;

  // Generate 1-3 random languages per version
  const numLanguages = (id % 3) + 1;
  const selectedLanguages = [];

  for (let i = 0; i < numLanguages; i++) {
    const langIndex = (id + i) % LANGUAGES.length;
    selectedLanguages.push(LANGUAGES[langIndex]);
  }

  for (const language of selectedLanguages) {
    // Generate pseudo-random but deterministic line counts based on id and language
    const seed = id + language.codePointAt(0);
    const codeLines = ((seed * 137) % 500) + 50; // 50-550 lines
    const commentLines = ((seed * 73) % 100) + 5; // 5-105 lines
    const files = ((seed * 29) % 8) + 1; // 1-8 files

    languages[language] = {
      code_lines: codeLines,
      comment_lines: commentLines,
      files: files,
    };

    totalCodeLines += codeLines;
    totalCommentLines += commentLines;
  }

  return {
    languages,
    total_code_lines: totalCodeLines,
    total_comment_lines: totalCommentLines,
  };
}
