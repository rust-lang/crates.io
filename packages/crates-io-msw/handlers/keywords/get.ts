import { db } from '../../index.js';
import { serializeKeyword } from '../../serializers/keyword.js';
import { notFound } from '../../utils/handlers.js';
import { http } from '../../utils/openapi-http.js';

export default http.get('/api/v1/keywords/{keyword}', ({ params, response }) => {
  let keyword = db.keyword.findFirst(q => q.where({ id: params.keyword }));
  if (!keyword) {
    return response.untyped(notFound());
  }

  return response(200).json({ keyword: serializeKeyword(keyword) });
});
