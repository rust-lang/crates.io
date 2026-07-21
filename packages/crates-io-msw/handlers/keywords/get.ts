import type { SuccessBody } from '../../utils/api-types.js';

import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { serializeKeyword } from '../../serializers/keyword.js';
import { notFound } from '../../utils/handlers.js';

export default http.get<{ keyword_id: string }>('/api/v1/keywords/:keyword_id', ({ params }) => {
  let keywordId = params.keyword_id;
  let keyword = db.keyword.findFirst(q => q.where({ id: keywordId }));
  if (!keyword) {
    return notFound();
  }

  return HttpResponse.json<SuccessBody<'find_keyword'>>({ keyword: serializeKeyword(keyword) });
});
