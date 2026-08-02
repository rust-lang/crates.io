import { db } from '../../index.js';
import { notFound } from '../../utils/handlers.js';
import { http } from '../../utils/openapi-http.js';

export default http.get('/api/v1/users/{id}/stats', ({ params, response }) => {
  let userId = parseInt(params.id);
  let user = db.user.findFirst(q => q.where({ id: userId }));
  if (!user) return response.untyped(notFound());

  let ownerships = db.crateOwnership.findMany(q => q.where(o => o.user?.id === userId));
  let total_downloads = ownerships.reduce((sum, o) => sum + o.crate.downloads, 0);

  return response(200).json({ total_downloads });
});
