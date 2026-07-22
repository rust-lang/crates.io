import type { SuccessBody } from '../../utils/api-types.js';

import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { serializeKeyword } from '../../serializers/keyword.js';
import { pageParams } from '../../utils/handlers.js';

export default http.get('/api/v1/keywords', ({ request }) => {
  let { skip, take } = pageParams(request);

  let keywords = db.keyword.findMany(undefined, { skip, take, orderBy: { keyword: 'asc' } });
  let total = db.keyword.count();

  return HttpResponse.json<SuccessBody<'list_keywords'>>({
    keywords: keywords.map(k => serializeKeyword(k)),
    meta: { total },
  });
});
