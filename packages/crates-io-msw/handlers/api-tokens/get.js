import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { serializeApiToken } from '../../serializers/api-token.js';
import { notFound } from '../../utils/handlers.js';
import { getSession } from '../../utils/session.js';

export default http.get('/api/v1/me/tokens/:tokenId', async ({ params }) => {
  let { user } = getSession();
  if (!user) {
    return HttpResponse.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
  }

  let { tokenId } = params;
  let token = db.apiToken.findFirst(q => q.where(token => token.id === parseInt(tokenId) && token.user.id === user.id));
  if (!token) return notFound();

  return HttpResponse.json({
    api_token: serializeApiToken(token),
  });
});
