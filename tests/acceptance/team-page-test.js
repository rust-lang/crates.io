import { test } from 'qunit';
import moduleForAcceptance from 'cargo/tests/helpers/module-for-acceptance';

moduleForAcceptance('Acceptance | team page');

test('has team organization display', async function(assert) {
    server.loadFixtures();

    await visit('/teams/github:org:thehydroimpulse');

    assert.dom('.team-info h1').hasText('org');
    assert.dom('.team-info h2').hasText('thehydroimpulseteam');
});

test('has link to github in team header', async function(assert) {
    server.loadFixtures();

    await visit('/teams/github:org:thehydroimpulse');

    assert.dom('.info a').hasAttribute('href', 'https://github.com/org_test');
});

test('github link has image in team header', async function(assert) {
    server.loadFixtures();

    await visit('/teams/github:org:thehydroimpulse');

    assert.dom('.info a img').hasAttribute('src', '/assets/GitHub-Mark-32px.png');
});

test('team organization details has github profile icon', async function(assert) {
    server.loadFixtures();

    await visit('/teams/github:org:thehydroimpulse');

    assert.dom('.info img').hasAttribute('src', 'https://avatars.githubusercontent.com/u/565790?v=3&s=170');
});
