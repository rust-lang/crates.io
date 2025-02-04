import { db } from '../index.js';
import { serializeModel } from '../utils/serializers.js';

export function serializeKeyword(keyword) {
  let serialized = serializeModel(keyword);

  serialized.crates_cnt = db.crate.count({ where: { keywords: { id: { equals: keyword.id } } } });

  return serialized;
}
