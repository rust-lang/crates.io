import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { getSession } from '../../utils/session.js';

export default http.delete('/api/v1/me/tokens/:tokenId', async ({ params }) => {
  let { user } = getSession();
  if (!user) {
    return HttpResponse.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
  }

  let { tokenId } = params;
  db.apiToken.delete({
    where: {
      id: { equals: parseInt(tokenId) },
      user: { id: { equals: user.id } },
    },
  });

  return HttpResponse.json({});
});
