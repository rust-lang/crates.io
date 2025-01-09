export function register(server) {
  server.get('https://docs.rs/crate/:crate/:version/status.json', function () {
    return {};
  });
}
