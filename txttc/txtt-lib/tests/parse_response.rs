/* The tests will be put to use again once the CLI is done
// Test on the output of the current binary when parsing.
// This will only work with the `test` binary.
const TEST_BIN: &'static str = "test";

// Different constants which might change if the
// error messages's contents  ever change (they likely will)
// Most of these could be taken from `src/parse.rs` but by
// copying them here instead, the following tests can
// stand fully on their own.
const KEY: &'static str = "key";
const CONSTANT: &'static str = "constant";
const OPTION: &'static str = "option";
const TEXT: &'static str = "text";
// `ContextMsg`s
const CONTAINS: &'static str = "Found invalid character(s) contained in ";
// const OPENS: &'static str = "Found invalid opening character for ";
const CLOSES: &'static str = "Found invalid closing character for ";
const EMPTY: &'static str = "Cannot process an empty input";
// `PossibleMsg`s
const MAYBE: &'static str = "Did you maybe mean ";
const ALLOWED: &'static str = "Allowed characters are ";
const FORBIDDEN: &'static str = "Forbidden characters are ";

// Make assertions on the behaviour of the binary
use assert_cmd::Command;
use self::helper::assert_out;

#[test]
fn invalid_idents() {
    // The ident of the key is interrupted by an invalid character
    assert_out("{nam*e}", vec![CLOSES, MAYBE, KEY]);
    assert_out("{}", vec![CONTAINS, ALLOWED, KEY]);
    assert_out("$---", vec![CONTAINS, ALLOWED, CONSTANT]);
    assert_out("${}", vec![CONTAINS, ALLOWED, OPTION]);
}

#[test]
fn invalid_text() {
    assert_out("$bla}", vec![CONTAINS, FORBIDDEN, TEXT]);
    assert_out("blalbla}bla", vec![CONTAINS, FORBIDDEN, TEXT]);
}

#[test]
fn empty_input() {
    assert_out("", vec![EMPTY]);
}

mod helper {
    use super::*;

    // Make assertions on the output of the binary
    // given an input
    pub fn assert_out(input: &str, contained: Vec<&str>) {
        for output in contained {
            Command::cargo_bin(TEST_BIN)
                .unwrap()
                .write_stdin(input)
                .assert()
                .stderr(predicates::str::contains(output));
        }
    }
}
*/
