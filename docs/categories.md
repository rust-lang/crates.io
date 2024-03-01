# Categories

Adding or editing the categories and corresponding descriptions displayed on [crates.io/categories](https://crates.io/categories) does not require a full development environment setup.

The list of categories available on crates.io is stored in [`src/boot/categories.toml`](https://github.com/rust-lang/crates.io/blob/main/src/boot/categories.toml). To propose adding, removing, or changing a category, send a pull request making the appropriate change to that file as noted in the comment at the top of the file. Please add a description that will help others to know what crates are in that category.

For new categories, it's helpful to note in your PR description examples of crates that would fit in that category, and describe what distinguishes the new category from existing categories.

After your PR is accepted, the next time that crates.io is deployed the categories will be synced from this file.
