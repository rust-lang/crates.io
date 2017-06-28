export default function(server) {
    server.loadFixtures('keywords');
    server.loadFixtures('teams');
    server.loadFixtures('users');
    server.loadFixtures('versions');
}
