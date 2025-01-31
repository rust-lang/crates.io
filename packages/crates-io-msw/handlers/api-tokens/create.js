import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { serializeApiToken } from '../../serializers/api-token.js';
import { getSession } from '../../utils/session.js';

export default http.put('/api/v1/me/tokens', async ({ request }) => {
  let { user } = getSession();
  if (!user) {
    return HttpResponse.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
  }

  let json = await request.json();

  let token = db.apiToken.create({
    user,
    name: json.api_token.name,
    crateScopes: json.api_token.crate_scopes ?? null,
    endpointScopes: json.api_token.endpoint_scopes ?? null,
    expiredAt: json.api_token.expired_at ?? null,
    createdAt: new Date().toISOString(),
  });

  return HttpResponse.json({
    api_token: serializeApiToken(token, { forCreate: true }),
  });
});
