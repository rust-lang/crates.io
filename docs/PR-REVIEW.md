# How to Review a Pull Request

Most Pull Requests (PRs) won't have everything mentioned in this document; take a look at the scope
of the change to determine which of the following apply.

## Does it work?

First and foremost, check out the branch locally and try it out!

- If it's a new feature, does it behave as the new feature is supposed to?
- If it's a bug fix, is the bug actually fixed?
- If it's a refactoring that shouldn't change behavior, is the behavior the same?
- Does it do whatever the connected issue says it should do?

So that's the happy path. Also try to break it!

- Can you give unexpected input? Too many, too few, or none of something?
- If you trigger an error condition, do the error messages make sense?
- What happens if you're logged in? Logged out?
- What if you reload the page?
- What if you use an old version of cargo?
- Does this work for both existing crates and newly published crates?
- What might make this code slow? Try that and see if it is slow.
- Does this code have any potential security problems?

If anything doesn't behave like it seems like it should, report this to the PR author using as much
detail as with a good bug report:

- Steps to reproduce
- What happened
- What you expected to happen

## Does it make sense?

If everything is working as you'd expect, next take a look at the code. Sometimes looking at the
code can give you ideas to test, so it's fine to cycle between reviewing the code and testing.

When reviewing the code, look for:

- Do you understand what the code is doing?
- Are names of variables and functions appropriately descriptive?
- Are error cases handled appropriately?
- Are there documentation comments for new types, functions, and methods?
- Are existing comments updated to match the code changes?
- Is there appropriate test coverage?
- Is there any way the new code would interact poorly with other features?
- Is there duplication that could be refactored?
- Is this change made in the right place or should this code be moved somewhere else?
- Could this code be made simpler?
- Is there anywhere else in the codebase that needs to change as a result of this change that was
  missed?
- Is there anything unexpected or unnecessary?
- Is there commented-out code, `println!` statements, or `console.log`s that were useful during
  debugging but should be removed?
- Do the commit messages accurately describe what is changed in each commit (and why those changes
  are necessary)? If you were debugging something and looking back at these commits wondering if
  they were the cause of a bug, would the commit messages be helpful?
- Are there commits merging upstream into this branch? These can be rebased out just before merging
  this branch in to master.

If any part of the code doesn't make sense, ask for an explanation! Be specific about what parts
are confusing, and perhaps make a guess as to what the code is doing for the PR author to correct.
As a maintainer, you have to maintain this code, so you need to be able to understand it!

Be constructive. Provide suggestions for improvement rather than just pointing out things that you
don't like. There are infinite ways to write code and the way you would have wrote it is likely not
objectively better or worse, unless the code doesn't do what it's supposed to do. Focus statements
on the code, not the developer.

If you see something awesome, comment on that too!

## Summarize

Write up a summary comment that lists the things you'd like to see changed before merging, if
applicable.
