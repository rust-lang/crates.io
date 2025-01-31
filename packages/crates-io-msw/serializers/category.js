import { db } from '../index.js';
import { serializeModel } from '../utils/serializers.js';

export function serializeCategory(category) {
  let serialized = serializeModel(category);

  serialized.crates_cnt ??= db.crate.count({ where: { categories: { id: { equals: category.id } } } });

  return serialized;
}

export function serializeCategorySlug(category) {
  return {
    id: category.id,
    slug: category.slug,
    description: category.description,
  };
}
