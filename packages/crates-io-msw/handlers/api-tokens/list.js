import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { serializeApiToken } from '../../serializers/api-token.js';
import { getSession } from '../../utils/session.js';

export default http.get('/api/v1/me/tokens', async ({ request }) => {
  let url = new URL(request.url);

  let { user } = getSession();
  if (!user) {
    return HttpResponse.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
  }

  let expiredAfter = new Date();
  if (url.searchParams.has('expired_days')) {
    expiredAfter.setUTCDate(expiredAfter.getUTCDate() - url.searchParams.get('expired_days'));
  }

  let apiTokens = db.apiToken
    .findMany({
      where: { user: { id: { equals: user.id } } },
      orderBy: { id: 'desc' },
    })
    .filter(token => !token.expiredAt || new Date(token.expiredAt) > expiredAfter);

  return HttpResponse.json({
    api_tokens: apiTokens.map(token => serializeApiToken(token)),
  });
});
