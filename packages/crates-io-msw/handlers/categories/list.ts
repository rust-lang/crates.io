import { db } from '../../index.js';
import { serializeCategory } from '../../serializers/category.js';
import { pageParams } from '../../utils/handlers.js';
import { http } from '../../utils/openapi-http.js';

export default http.get('/api/v1/categories', ({ request, response }) => {
  let { skip, take } = pageParams(request);

  let categories = db.category.findMany(undefined, { skip, take, orderBy: { category: 'asc' } });
  let total = db.category.count();

  return response(200).json({
    categories: categories.map(c => serializeCategory(c)),
    meta: { total },
  });
});
