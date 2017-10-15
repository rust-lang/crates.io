import { test } from 'qunit';
import moduleForAcceptance from 'cargo/tests/helpers/module-for-acceptance';

moduleForAcceptance('Acceptance | user page');

test('has user display', async function(assert) {
    server.loadFixtures();

    await visit('/users/thehydroimpulse');

    assert.dom('#crates-heading h1').hasText('thehydroimpulse');
});

test('has link to github in user header', async function(assert) {
    server.loadFixtures();

    await visit('/users/thehydroimpulse');

    assert.dom('#crates-heading a').hasAttribute('href', 'https://github.com/thehydroimpulse');
});

test('github link has image in user header', async function(assert) {
    server.loadFixtures();

    await visit('/users/thehydroimpulse');

    assert.dom('#crates-heading a img').hasAttribute('src', '/assets/GitHub-Mark-32px.png');
});

test('user details has github profile icon', async function(assert) {
    server.loadFixtures();

    await visit('/users/thehydroimpulse');

    assert.dom('#crates-heading img').hasAttribute('src', 'https://avatars.githubusercontent.com/u/565790?v=3&s=170');
});
