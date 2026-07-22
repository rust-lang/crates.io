import type { components } from '@crates-io/api-client';
import type { ApiToken as MswApiToken } from '../models/index.js';

type ApiToken = components['schemas']['ApiToken'];
type ApiTokenWithToken = components['schemas']['ApiTokenWithToken'];

export function serializeApiToken(token: MswApiToken): ApiToken;
export function serializeApiToken(token: MswApiToken, options: { forCreate?: false }): ApiToken;
export function serializeApiToken(token: MswApiToken, options: { forCreate: true }): ApiTokenWithToken;
export function serializeApiToken(token: MswApiToken, { forCreate = false } = {}): ApiToken | ApiTokenWithToken {
  let serialized: ApiToken = {
    id: token.id,
    name: token.name,
    created_at: new Date(token.createdAt).toISOString(),
    crate_scopes: token.crateScopes,
    endpoint_scopes: token.endpointScopes,
    expired_at: token.expiredAt ? new Date(token.expiredAt).toISOString() : null,
    last_used_at: token.lastUsedAt ? new Date(token.lastUsedAt).toISOString() : null,
  };

  if (forCreate) {
    return { ...serialized, token: token.token };
  }

  return serialized;
}
