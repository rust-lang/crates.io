export default function(server) {
    server.loadFixtures('categories');
    server.loadFixtures('crates');
    server.loadFixtures('dependencies');
    server.loadFixtures('keywords');
    server.loadFixtures('teams');
    server.loadFixtures('users');
    server.loadFixtures('versions');
}
