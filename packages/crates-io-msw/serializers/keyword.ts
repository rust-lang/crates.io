import type { components } from '@crates-io/api-client';
import type { Keyword } from '../models/index.js';

import { db } from '../index.js';

type ApiKeyword = components['schemas']['Keyword'];

export function serializeKeyword(keyword: Keyword): ApiKeyword {
  let crateCount = db.crate.findMany(q => q.where(crate => crate.keywords.some(k => k.id === keyword.id))).length;

  return {
    id: keyword.id,
    keyword: keyword.keyword,
    created_at: keyword.created_at,
    crates_cnt: crateCount,
  };
}
