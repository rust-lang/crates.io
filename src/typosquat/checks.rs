use typomania::checks::{Check, Squat};
use typomania::{Corpus, Package};

/// A typomania check that checks if commonly used prefixes or suffixes have been added to or
/// removed from a package name.
pub struct Affixes {
    affixes: Vec<String>,
    separators: Vec<String>,
}

impl Affixes {
    pub fn new<Aff, Sep>(affixes: Aff, separators: Sep) -> Self
    where
        Aff: Iterator,
        Aff::Item: ToString,
        Sep: Iterator,
        Sep::Item: ToString,
    {
        Self {
            affixes: affixes.map(|s| s.to_string()).collect(),
            separators: separators.map(|s| s.to_string()).collect(),
        }
    }
}

impl Check for Affixes {
    fn check(
        &self,
        corpus: &dyn Corpus,
        name: &str,
        package: &dyn Package,
    ) -> typomania::Result<Vec<Squat>> {
        let mut squats = Vec::new();

        for separator in self.separators.iter() {
            for affix in self.affixes.iter() {
                // If the package being examined starts with this prefix and separator combo, then
                // we should see if it exists without that prefix in the popular crate corpus.
                let combo = format!("{affix}{separator}");
                if let Some(stem) = name.strip_prefix(&combo) {
                    if corpus.possible_squat(stem, name, package)? {
                        squats.push(Squat::Custom {
                            message: format!("adds the {combo} prefix"),
                            package: stem.to_string(),
                        })
                    }
                }

                // Alternatively, let's see if adding the prefix and separator combo to the package
                // results in something popular; eg somebody trying to squat `foo` with `rs-foo`.
                let prefixed = format!("{combo}{name}");
                if corpus.possible_squat(&prefixed, name, package)? {
                    squats.push(Squat::Custom {
                        message: format!("removes the {combo} prefix"),
                        package: prefixed,
                    });
                }

                // If the package being examined ends in this separator and suffix combo, then we
                // should see if it exists without that suffix in the popular crate corpus.
                let combo = format!("{separator}{affix}");
                if let Some(stem) = name.strip_suffix(&combo) {
                    if corpus.possible_squat(stem, name, package)? {
                        squats.push(Squat::Custom {
                            message: format!("adds the {combo} suffix"),
                            package: stem.to_string(),
                        })
                    }
                }

                // Alternatively, let's see if adding the separator and suffix combo to the package
                // results in something popular; eg somebody trying to squat `foo` with `foo-rs`.
                let suffixed = format!("{name}{combo}");
                if corpus.possible_squat(&suffixed, name, package)? {
                    squats.push(Squat::Custom {
                        message: format!("removes the {combo} suffix"),
                        package: suffixed,
                    });
                }
            }
        }

        Ok(squats)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use googletest::prelude::*;
    use typomania::{AuthorSet, Harness};

    use super::*;

    #[test]
    fn test_affixes() -> anyhow::Result<()> {
        let popular = TestCorpus::default()
            .with_package(TestPackage::new("foo", "foo", ["Alice", "Bob"]))
            .with_package(TestPackage::new("bar-rs", "Rust bar", ["Charlie"]))
            .with_package(TestPackage::new("quux_sys", "libquux", ["Alice"]))
            .with_package(TestPackage::new("core-xyz", "Core xyz", ["Alice"]));

        let harness = Harness::empty_builder()
            .with_check(Affixes::new(
                ["core", "rs", "sys"].iter(),
                ["-", "_"].iter(),
            ))
            .build(popular);

        // Try some packages that shouldn't be squatting anything.
        for package in [
            TestPackage::new("bar", "shared author", ["Charlie"]),
            TestPackage::new("baz", "unrelated package", ["Bob"]),
            TestPackage::new("foo-rs", "shared author", ["Alice"]),
        ]
        .into_iter()
        {
            let name = package.name.clone();
            let squats = harness.check_package(&name, Box::new(package))?;
            assert_that!(squats, empty());
        }

        // Now try some packages that should be.
        for package in [
            TestPackage::new("foo-rs", "no shared author", ["Charlie"]),
            TestPackage::new("rs-foo", "no shared author", ["Charlie"]),
            TestPackage::new("quux", "libquux", ["Charlie"]),
            TestPackage::new("quux_sys_rs", "libquux... for Rust?", ["Charlie"]),
            TestPackage::new("xyz", "unprefixed core-xyz", ["Charlie"]),
        ]
        .into_iter()
        {
            let name = package.name.clone();
            let squats = harness.check_package(&name, Box::new(package))?;
            assert_that!(squats, not(empty()));
        }

        Ok(())
    }

    struct TestPackage {
        name: String,
        description: String,
        authors: HashSet<String>,
    }

    impl TestPackage {
        fn new(name: &str, description: &str, authors: impl AsRef<[&'static str]>) -> Self {
            Self {
                name: name.to_string(),
                description: description.to_string(),
                authors: authors.as_ref().iter().map(|a| a.to_string()).collect(),
            }
        }
    }

    impl Package for TestPackage {
        fn authors(&self) -> &dyn AuthorSet {
            self
        }

        fn description(&self) -> Option<&str> {
            Some(&self.description)
        }

        fn shared_authors(&self, other: &dyn AuthorSet) -> bool {
            self.authors.iter().any(|author| other.contains(author))
        }
    }

    impl AuthorSet for TestPackage {
        fn contains(&self, author: &str) -> bool {
            self.authors.contains(author)
        }
    }

    #[derive(Default)]
    struct TestCorpus(HashMap<String, TestPackage>);

    impl TestCorpus {
        fn with_package(mut self, package: TestPackage) -> Self {
            self.0.insert(package.name.clone(), package);
            self
        }
    }

    impl Corpus for TestCorpus {
        fn contains_name(&self, name: &str) -> typomania::Result<bool> {
            Ok(self.0.contains_key(name))
        }

        fn get(&self, name: &str) -> typomania::Result<Option<&dyn Package>> {
            Ok(self.0.get(name).map(|tp| tp as &dyn Package))
        }
    }
}
