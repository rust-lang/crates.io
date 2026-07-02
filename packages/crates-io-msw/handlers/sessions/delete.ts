import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';

export default http.delete('/api/private/session', () => {
  db.mswSession.deleteMany(q => q.where(() => true));
  return HttpResponse.json({ ok: true });
});
