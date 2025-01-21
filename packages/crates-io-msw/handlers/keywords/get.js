import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { serializeKeyword } from '../../serializers/keyword.js';
import { notFound } from '../../utils/handlers.js';

export default http.get('/api/v1/keywords/:keyword_id', ({ params }) => {
  let keywordId = params.keyword_id;
  let keyword = db.keyword.findFirst({ where: { id: { equals: keywordId } } });
  if (!keyword) {
    return notFound();
  }

  return HttpResponse.json({ keyword: serializeKeyword(keyword) });
});
