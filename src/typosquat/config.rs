//! Things that should really be read from a configuration file, but are just hardcoded while we
//! experiment with this.

/// Valid characters in crate names.
pub(super) static CRATE_NAME_ALPHABET: &str =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz1234567890-_";

/// Commonly used separators when building crate names.
pub(super) static SUFFIX_SEPARATORS: &[&str] = &["-", "_"];

/// Commonly used suffixes when building crate names.
pub(super) static SUFFIXES: &[&str] = &["api", "cargo", "cli", "core", "lib", "rs", "rust", "sys"];

/// The number of crates to consider in the "top crates" corpus.
pub(super) static TOP_CRATES: i64 = 3000;

/// This is based on a pre-existing list we've used with crates.io for "easily confused
/// characters". This is a mixture of visual substitutions and typos on QWERTY, QWERTZ, and AZERTY
/// keyboards.
pub(super) static TYPOS: &[(char, &[&str])] = &[
    ('1', &["2", "q", "i", "l"]),
    ('2', &["1", "q", "w", "3"]),
    ('3', &["2", "w", "e", "4"]),
    ('4', &["3", "e", "r", "5"]),
    ('5', &["4", "r", "t", "6", "s"]),
    ('6', &["5", "t", "y", "7"]),
    ('7', &["6", "y", "u", "8"]),
    ('8', &["7", "u", "i", "9"]),
    ('9', &["8", "i", "o", "0"]),
    ('0', &["9", "o", "p", "-"]),
    ('-', &["_", "0", "p", ".", ""]),
    ('_', &["-", "0", "p", ".", ""]),
    ('q', &["1", "2", "w", "a", "s", "z"]),
    ('w', &["2", "3", "e", "s", "a", "q", "vv", "x"]),
    ('e', &["3", "4", "r", "d", "s", "w", "z"]),
    ('r', &["4", "5", "t", "f", "d", "e"]),
    ('t', &["5", "6", "y", "g", "f", "r"]),
    ('y', &["6", "7", "u", "h", "t", "i", "a", "s", "x"]),
    ('u', &["7", "8", "i", "j", "y", "v"]),
    ('i', &["1", "8", "9", "o", "l", "k", "j", "u", "y"]),
    ('o', &["9", "0", "p", "l", "i"]),
    ('p', &["0", "-", "o"]),
    ('a', &["q", "w", "s", "z", "1", "2"]),
    ('s', &["w", "d", "x", "z", "a", "5", "q"]),
    ('d', &["e", "r", "f", "c", "x", "s"]),
    ('f', &["r", "g", "v", "c", "d"]),
    ('g', &["t", "h", "b", "v", "f"]),
    ('h', &["y", "j", "n", "b", "g"]),
    ('j', &["u", "i", "k", "m", "n", "h"]),
    ('k', &["i", "o", "l", "m", "j"]),
    ('l', &["i", "o", "p", "k", "1"]),
    (
        'z',
        &["a", "s", "x", "6", "7", "u", "h", "t", "i", "e", "2", "3"],
    ),
    ('x', &["z", "s", "d", "c", "w"]),
    ('c', &["x", "d", "f", "v"]),
    ('v', &["c", "f", "g", "b", "u"]),
    ('b', &["v", "g", "h", "n"]),
    ('n', &["b", "h", "j", "m"]),
    ('m', &["n", "j", "k", "rn"]),
    ('.', &["-", "_", ""]),
];
