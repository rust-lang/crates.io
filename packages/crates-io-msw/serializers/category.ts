import type { Category } from '../models/index.js';

import { db } from '../index.js';
import { serializeModel } from '../utils/serializers.js';

export function serializeCategory(category: Category) {
  let serialized = serializeModel(category);

  let crateCount = db.crate.findMany(q => q.where(crate => crate.categories.some(c => c.id === category.id))).length;
  serialized.crates_cnt ??= crateCount;

  return serialized;
}

export function serializeCategorySlug(category: Category) {
  return {
    id: category.id,
    slug: category.slug,
    description: category.description,
  };
}
