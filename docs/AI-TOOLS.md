# Guidelines for using AI tools

The person submitting an issue or PR is responsible for its content,
regardless of whether AI tools were used in its creation. Generative AI
tools can produce output quickly, but discretion, good judgment, and
critical thinking are the foundation of all good contributions. We value
good code, concise accurate documentation, and well scoped PRs without
unneeded code churn.

## Considerations for success

Authors must review the work done by AI tooling in detail to ensure it
actually makes sense before proposing it as a PR or filing it as an issue.

We expect PR authors and those filing issues to be able to explain their
proposed changes in their own words.

Disclosure of the use of AI tools in the PR description is appreciated,
while not required. Be prepared to explain how the tool was used and what
changes it made.

Whether you are using AI tools or not, keep the following principles in
mind for the quality of your contribution:

- Consider whether the change is necessary
- Make minimal, focused changes
- Follow existing coding style and patterns
- Write tests that exercise the change
- Keep backwards compatibility with prior releases in mind. Existing
  tests may be ensuring specific API behaviors are maintained.

Pay close attention to AI generated recommendations for testing changes.
Provide input about Python's testing principles when guiding an AI model.
Always review the output before opening a pull request or issue,
including proposed PR or issue titles and descriptions.

## Acceptable uses

Some of the acceptable uses of generative AI include:

- Assistance with writing comments, especially in a non-native language
- Gaining understanding of existing code
- Supplementing contributor knowledge for code, tests, and documentation

## Unacceptable uses

Maintainers may close issues and PRs that are not useful or productive,
regardless of whether AI tools were used or not.

If a contributor repeatedly opens unproductive issues or PRs, they may be
blocked from contributing to the project because it is disruptive and
disrespectful of the maintainers time.

It is not acceptable to alter or bypass existing tests, or remove desired
functionality, in order to make a failing test pass. Such changes are not
a real fix.
