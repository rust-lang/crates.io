import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { serializeCategory } from '../../serializers/category.js';
import { notFound } from '../../utils/handlers.js';

export default http.get('/api/v1/categories/:category_id', ({ params }) => {
  let catId = params.category_id;
  let category = db.category.findFirst(q => q.where({ id: catId }));
  if (!category) {
    return notFound();
  }

  return HttpResponse.json({ category: serializeCategory(category) });
});
