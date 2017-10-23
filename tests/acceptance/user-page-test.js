import { test } from 'qunit';
import { visit } from 'ember-native-dom-helpers';
import moduleForAcceptance from 'cargo/tests/helpers/module-for-acceptance';

moduleForAcceptance('Acceptance | user page');

test('has user display', async function(assert) {
    server.loadFixtures();

    await visit('/users/thehydroimpulse');

    assert.dom('[data-test-heading] [data-test-username]').hasText('thehydroimpulse');
});

test('has link to github in user header', async function(assert) {
    server.loadFixtures();

    await visit('/users/thehydroimpulse');

    assert.dom('[data-test-heading] [data-test-user-link]')
        .hasAttribute('href', 'https://github.com/thehydroimpulse');
});

test('github link has image in user header', async function(assert) {
    server.loadFixtures();

    await visit('/users/thehydroimpulse');

    assert.dom('[data-test-heading] [data-test-user-link] img')
        .hasAttribute('src', '/assets/GitHub-Mark-32px.png');
});

test('user details has github profile icon', async function(assert) {
    server.loadFixtures();

    await visit('/users/thehydroimpulse');

    assert.dom('[data-test-heading] [data-test-avatar]')
        .hasAttribute('src', 'https://avatars.githubusercontent.com/u/565790?v=3&s=170');
});
