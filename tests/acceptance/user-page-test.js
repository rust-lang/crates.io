import { test } from 'qunit';
import moduleForAcceptance from 'cargo/tests/helpers/module-for-acceptance';

moduleForAcceptance('Acceptance | user page');

test('has user display', function(assert) {
    visit('/users/thehydroimpulse');

    andThen(function() {
        hasText(assert, '#crates-heading h1', 'thehydroimpulse');
    });
});

test('has link to github in user header', function(assert) {
    visit('/users/thehydroimpulse');

    andThen(function() {
        const $githubLink = findWithAssert('#crates-heading a');
        assert.equal($githubLink.attr('href').trim(), 'https://github.com/thehydroimpulse');
    });
});

test('github link has image in user header', function(assert) {
    visit('/users/thehydroimpulse');

    andThen(function() {
        const $githubImg = findWithAssert('#crates-heading a img');
        assert.equal($githubImg.attr('src').trim(), '/assets/GitHub-Mark-32px.png');
    });
});

test('user details has github profile icon', function(assert) {
    visit('/users/thehydroimpulse');

    andThen(function() {
        const $githubProfileImg = findWithAssert('#crates-heading img');
        assert.equal($githubProfileImg.attr('src').trim(), 'https://avatars.githubusercontent.com/u/565790?v=3&s=170');
    });
});
