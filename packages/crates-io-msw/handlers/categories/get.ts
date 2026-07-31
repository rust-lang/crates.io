import { db } from '../../index.js';
import { serializeCategory } from '../../serializers/category.js';
import { notFound } from '../../utils/handlers.js';
import { http } from '../../utils/openapi-http.js';

export default http.get('/api/v1/categories/{category}', ({ params, response }) => {
  let catId = params.category;
  let category = db.category.findFirst(q => q.where({ id: catId }));
  if (!category) {
    return response.untyped(notFound());
  }

  return response(200).json({ category: serializeCategory(category) });
});
