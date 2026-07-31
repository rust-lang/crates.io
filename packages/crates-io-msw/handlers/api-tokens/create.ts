import type { SuccessBody } from '../../utils/api-types.js';

import { endpointScopeValues } from '@crates-io/api-client';
import { http, HttpResponse } from 'msw';
import * as v from 'valibot';

import { db } from '../../index.js';
import { serializeApiToken } from '../../serializers/api-token.js';
import { getSession } from '../../utils/session.js';

const endpointScopesSchema = v.nullish(v.array(v.picklist(endpointScopeValues)));

export default http.put('/api/v1/me/tokens', async ({ request }) => {
  let { user } = getSession();
  if (!user) {
    return HttpResponse.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
  }

  let json = (await request.json()) as {
    api_token: {
      name: string;
      crate_scopes?: string[] | null;
      endpoint_scopes?: string[] | null;
      expired_at?: string | null;
    };
  };

  let endpointScopes = v.safeParse(endpointScopesSchema, json.api_token.endpoint_scopes);
  if (!endpointScopes.success) {
    return HttpResponse.json({ errors: [{ detail: 'invalid endpoint scope' }] }, { status: 400 });
  }

  let token = await db.apiToken.create({
    user,
    name: json.api_token.name,
    crateScopes: json.api_token.crate_scopes ?? null,
    endpointScopes: endpointScopes.output ?? null,
    expiredAt: json.api_token.expired_at ?? null,
    createdAt: new Date().toISOString(),
  });

  return HttpResponse.json<SuccessBody<'create_api_token'>>({
    api_token: serializeApiToken(token, { forCreate: true }),
  });
});
