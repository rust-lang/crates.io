import type { components } from '@crates-io/api-client';
import type { Category } from '../models/index.js';

import { db } from '../index.js';

type ApiCategory = components['schemas']['Category'];
type ApiCategorySlug = components['schemas']['Slug'];

export function serializeCategory(category: Category): ApiCategory {
  let crateCount = db.crate.findMany(q => q.where(crate => crate.categories.some(c => c.id === category.id))).length;

  return {
    id: category.id,
    category: category.category,
    slug: category.slug,
    description: category.description,
    created_at: category.created_at,
    crates_cnt: category.crates_cnt ?? crateCount,
  };
}

export function serializeCategorySlug(category: Category): ApiCategorySlug {
  return {
    id: category.id,
    slug: category.slug,
    description: category.description,
  };
}
