import { db } from '../index.js';
import { serializeModel } from '../utils/serializers.js';

export function serializeKeyword(keyword) {
  let serialized = serializeModel(keyword);

  serialized.crates_cnt = db.crate.findMany(q =>
    q.where(crate => crate.keywords.some(k => k.id === keyword.id)),
  ).length;

  return serialized;
}
