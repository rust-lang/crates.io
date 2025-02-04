import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { serializeCategorySlug } from '../../serializers/category.js';

export default http.get('/api/v1/category_slugs', () => {
  let allCategories = db.category.findMany({ orderBy: { category: 'asc' } });

  return HttpResponse.json({ category_slugs: allCategories.map(c => serializeCategorySlug(c)) });
});
