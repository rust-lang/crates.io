import { db } from '../../index.js';
import { serializeKeyword } from '../../serializers/keyword.js';
import { pageParams } from '../../utils/handlers.js';
import { http } from '../../utils/openapi-http.js';

export default http.get('/api/v1/keywords', ({ request, response }) => {
  let { skip, take } = pageParams(request);

  let keywords = db.keyword.findMany(undefined, { skip, take, orderBy: { keyword: 'asc' } });
  let total = db.keyword.count();

  return response(200).json({
    keywords: keywords.map(k => serializeKeyword(k)),
    meta: { total },
  });
});
