import { db } from '../../index.js';
import { serializeCategorySlug } from '../../serializers/category.js';
import { http } from '../../utils/openapi-http.js';

export default http.get('/api/v1/category_slugs', ({ response }) => {
  let allCategories = db.category.findMany(undefined, { orderBy: { category: 'asc' } });

  return response(200).json({
    category_slugs: allCategories.map(c => serializeCategorySlug(c)),
  });
});
