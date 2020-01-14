export function register(server) {
  server.get('https://docs.rs/crate/:crate/:version/builds.json', function() {
    return [];
  });
}
