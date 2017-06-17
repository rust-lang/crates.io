import { test } from 'qunit';
import moduleForAcceptance from 'cargo/tests/helpers/module-for-acceptance';

moduleForAcceptance('Acceptance | user page');

test('has user display', async function(assert) {
    await visit('/users/thehydroimpulse');

    hasText(assert, '#crates-heading h1', 'thehydroimpulse');
});

test('has link to github in user header', async function(assert) {
    await visit('/users/thehydroimpulse');

    const $githubLink = findWithAssert('#crates-heading a');
    assert.equal($githubLink.attr('href').trim(), 'https://github.com/thehydroimpulse');
});

test('github link has image in user header', async function(assert) {
    await visit('/users/thehydroimpulse');

    const $githubImg = findWithAssert('#crates-heading a img');
    assert.equal($githubImg.attr('src').trim(), '/assets/GitHub-Mark-32px.png');
});

test('user details has github profile icon', async function(assert) {
    await visit('/users/thehydroimpulse');

    const $githubProfileImg = findWithAssert('#crates-heading img');
    assert.equal($githubProfileImg.attr('src').trim(), 'https://avatars.githubusercontent.com/u/565790?v=3&s=170');
});
