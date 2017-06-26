export default function matchesText(assert, selector, expectedRegex) {
    const $selected = findWithAssert(selector);
    const $actual = $selected.text().trim().replace(/\s+/g, ' ');
    assert.notEqual(
        null,
        $actual.match(expectedRegex),
        `Text found ('${$actual}') did not match regex ('${expectedRegex}')`
    );
}
