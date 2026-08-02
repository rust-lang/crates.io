import { db } from '../../index.js';
import { serializeApiToken } from '../../serializers/api-token.js';
import { http } from '../../utils/openapi-http.js';
import { getSession } from '../../utils/session.js';

export default http.get('/api/v1/me/tokens', ({ query, response }) => {
  let { user } = getSession();
  if (!user) {
    return response.untyped(
      Response.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 }),
    );
  }

  let expiredAfter = new Date();
  let expiredDays = query.get('expired_days');
  if (expiredDays !== null) {
    expiredAfter.setUTCDate(expiredAfter.getUTCDate() - Number(expiredDays));
  }

  let apiTokens = db.apiToken
    .findMany(q => q.where(token => token.user.id === user.id), { orderBy: { id: 'desc' } })
    .filter(token => !token.expiredAt || new Date(token.expiredAt) > expiredAfter);

  return response(200).json({
    api_tokens: apiTokens.map(token => serializeApiToken(token)),
  });
});
