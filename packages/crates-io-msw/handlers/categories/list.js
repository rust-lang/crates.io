import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { serializeCategory } from '../../serializers/category.js';
import { pageParams } from '../../utils/handlers.js';

export default http.get('/api/v1/categories', ({ request }) => {
  let { skip, take } = pageParams(request);

  let categories = db.category.findMany({ skip, take, orderBy: { category: 'asc' } });
  let total = db.category.count();

  return HttpResponse.json({ categories: categories.map(c => serializeCategory(c)), meta: { total } });
});
