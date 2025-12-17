import { Collection } from '@msw/data';
import * as v from 'valibot';

import * as counters from '../utils/counters.js';

const LICENSES = ['MIT/Apache-2.0', 'MIT', 'Apache-2.0'];

const LANGUAGES = ['Rust', 'JavaScript', 'TypeScript', 'Python', 'CSS', 'HTML', 'Shell'];

const schema = v.pipe(
  v.object({
    id: v.optional(v.number()),

    num: v.optional(v.string()),
    created_at: v.optional(v.string(), '2010-06-16T21:30:45Z'),
    updated_at: v.optional(v.string(), '2017-02-24T12:34:56Z'),
    yanked: v.optional(v.boolean(), false),
    yank_message: v.optional(v.nullable(v.string()), null),
    license: v.optional(v.string()),
    downloads: v.optional(v.number()),
    features: v.optional(v.record(v.string(), v.any()), {}),
    crate_size: v.optional(v.number()),
    readme: v.optional(v.nullable(v.string()), null),
    rust_version: v.optional(v.nullable(v.string()), null),
    trustpub_data: v.optional(v.any(), null),
    linecounts: v.optional(v.any()),

    crate: v.any(),
    publishedBy: v.optional(v.any(), null),
  }),
  v.transform(function (input) {
    let counter = counters.increment('version');
    let id = input.id ?? counter;
    let num = input.num ?? `1.0.${id - 1}`;
    let license = input.license ?? LICENSES[id % LICENSES.length];
    let downloads = input.downloads ?? (((id + 13) * 42) % 13) * 1234;
    let crate_size = input.crate_size ?? (((id + 13) * 42) % 13) * 54_321;
    let linecounts = input.linecounts ?? generateLinecounts(id);
    return { ...input, id, num, license, downloads, crate_size, linecounts };
  }),
);

const collection = new Collection({ schema });

export default collection;

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
