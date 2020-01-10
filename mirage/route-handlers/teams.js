import { notFound } from './-utils';

export function register(server) {
  server.get('/api/v1/teams/:team_id', (schema, request) => {
    let login = request.params.team_id;
    let team = schema.teams.findBy({ login });
    return team ? team : notFound();
  });
}
