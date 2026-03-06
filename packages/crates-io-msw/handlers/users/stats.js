import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { notFound } from '../../utils/handlers.js';

export default http.get('/api/v1/users/:user_id/stats', ({ params }) => {
  let userId = parseInt(params.user_id);
  let user = db.user.findFirst(q => q.where({ id: userId }));
  if (!user) return notFound();

  let ownerships = db.crateOwnership.findMany(q => q.where(o => o.user?.id === userId));
  let total_downloads = ownerships.reduce((sum, o) => sum + o.crate.downloads, 0);

  return HttpResponse.json({ total_downloads });
});
