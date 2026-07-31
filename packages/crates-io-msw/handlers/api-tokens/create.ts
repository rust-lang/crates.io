import { endpointScopeValues } from '@crates-io/api-client';
import * as v from 'valibot';

import { db } from '../../index.js';
import { serializeApiToken } from '../../serializers/api-token.js';
import { http } from '../../utils/openapi-http.js';
import { getSession } from '../../utils/session.js';

const endpointScopesSchema = v.nullish(v.array(v.picklist(endpointScopeValues)));

export default http.put('/api/v1/me/tokens', async ({ request, response }) => {
  let { user } = getSession();
  if (!user) {
    return response.untyped(
      Response.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 }),
    );
  }

  let json = await request.json();

  let endpointScopes = v.safeParse(endpointScopesSchema, json.api_token.endpoint_scopes);
  if (!endpointScopes.success) {
    return response.untyped(Response.json({ errors: [{ detail: 'invalid endpoint scope' }] }, { status: 400 }));
  }

  let token = await db.apiToken.create({
    user,
    name: json.api_token.name,
    crateScopes: json.api_token.crate_scopes ?? null,
    endpointScopes: endpointScopes.output ?? null,
    expiredAt: json.api_token.expired_at ?? null,
    createdAt: new Date().toISOString(),
  });

  return response(200).json({
    api_token: serializeApiToken(token, { forCreate: true }),
  });
});
