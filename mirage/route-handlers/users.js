import { notFound } from './-utils';

export function register(server) {
  server.get('/api/v1/users/:user_id', (schema, request) => {
    let login = request.params.user_id;
    let user = schema.users.findBy({ login });
    return user ? user : notFound();
  });
}
