export default function hasText(assert, selector, expected) {
    assert.equal(findWithAssert(selector).text().trim().replace(/\s+/g, ' '), expected);
}
