#!/usr/bin/env bash
# Tests for scripts/comment-drift.sh, written alongside it. Each case commits
# fixtures to a scratch git repository, stages a change and compares the list
# with the one expected. The same file is in each project beside the script.
# Not run by the hook; run it after changing the script:
#   scripts/test-comment-drift.sh
# It prints ok or FAIL per case, and exits 1 if any failed.
set -euo pipefail
drift="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/comment-drift.sh"
repo=$(mktemp -d)
trap 'rm -rf "$repo"' EXIT
cd "$repo"
git init -q
git config user.name test
git config user.email test@example.invalid
git config commit.gpgsign false
mkdir .no-hooks
git config core.hooksPath .no-hooks

failed=0
# check <name> [script args…] < expected output
check() {
    local name=$1 expected actual status=0
    shift
    expected=$(cat)
    actual=$("$drift" "$@" 2>&1) || status=$?
    [[ $status -eq 0 ]] || actual+=$'\n'"(exit $status)"
    if [[ "$actual" == "$expected" ]]; then
        echo "ok   $name"
    else
        echo "FAIL $name"
        diff <(echo "$expected") <(echo "$actual") | sed 's/^/     /' || true
        failed=1
    fi
}
commit() { git add -A && git commit -qm "$1"; }

# The fixtures before the changes.
cat >inside.rs <<'EOF'
fn gain(x: f32) -> f32 {
    // Half, so two voices can't clip (B-AMP-1).
    let y = x * 0.5;
    y /* trailing */
}

fn other() {
    // Not touched: not listed.
}
EOF
cat >doc.rs <<'EOF'
/// Doubles `x`.
#[inline]
fn double(x: f32) -> f32 {
    x * 2.0
}
EOF
cat >edited.rs <<'EOF'
fn limit(x: f32) -> f32 {
    // The limit is 3 dB (B-LIM-1).
    let l = 1.41;
    // Unchanged.
    x.min(l)
}
EOF
cat >outside.rs <<'EOF'
//! The file's header: not listed for a change below it.
#![allow(dead_code)]
use std::f32::consts::PI;

/// The ceiling, -1 dBFS (B-LIM-2).
const CEILING: f32 = 0.89;

// A table, its entries commented.
const TABLE: [f32; 3] = [
    1.0, // unity
    2.0, // double
    3.0,
];
EOF
cat >header.rs <<'EOF'
//! A header directly above a statement: still the file's, not listed.
use std::fmt;
EOF
cat >nested.rs <<'EOF'
/// The module's doc.
mod inner {
    // The module's own comment.
    pub struct S;

    /// The impl's doc.
    impl S {
        // Between methods.
        const N: usize = 4; // four lanes

        /// f's doc.
        fn f(&self) -> usize {
            // In f, before the nested fn.
            fn helper(x: usize) -> usize {
                // In helper.
                x + 1
            }
            // In f, after.
            helper(Self::N)
        }

        /// g's doc.
        fn g(&self) {}

        /// h's doc.
        fn h(&self) {
            // In h.
            let _ = 1;
        }
    }
}
EOF
cat >lexing.rs <<'EOF'
/// Literals that look like comments or braces.
fn tricky<'a>(s: &'a str, xs: [u8; 2]) -> usize {
    let a = "// not a comment { ";
    let b = r#"also "not" /* a comment */ }"#;
    let c = '{';
    let d = b'}';
    /* a block /* nested */ comment
       over two lines */
    let e = '\'';
    // The real comment.
    s.len() + a.len() + b.len() + (c as usize) + (d as usize) + (e as usize) + xs.len()
}

/// Untouched, so not listed.
fn after() {}
EOF
cat >gone.rs <<'EOF'
/// A file to delete.
fn gone() {
    // Its comment.
}
EOF
cat >shrink.rs <<'EOF'
use std::fmt;
use std::io;
use std::mem;

fn shrink() {
    // Kept.
    let a = 1;
    let b = 2;
    let _ = a;
}

/// Next door.
fn next() {}
EOF
cat >notes.md <<'EOF'
// Markdown isn't Rust: never listed.
EOF
commit "fixtures"

# 1. A changed function's unchanged comments are listed, a trailing comment
#    and a block comment among them; an untouched function's are not.
sed -i 's/x \* 0.5/x * 0.25/' inside.rs
git add inside.rs
check "a changed function lists its unchanged comments" <<'EOF'
== inside.rs
-- fn gain(x: f32) -> f32 (lines 1-5)
inside.rs:2: // Half, so two voices can't clip (B-AMP-1).
inside.rs:4: y /* trailing */
EOF
# The same, committed and named by a base (a reviewer's use), with and
# without a head.
commit "1"
check "a base lists the changes since it" HEAD~1 <<'EOF'
== inside.rs
-- fn gain(x: f32) -> f32 (lines 1-5)
inside.rs:2: // Half, so two voices can't clip (B-AMP-1).
inside.rs:4: y /* trailing */
EOF
check "a base and a head" HEAD~1 HEAD <<'EOF'
== inside.rs
-- fn gain(x: f32) -> f32 (lines 1-5)
inside.rs:2: // Half, so two voices can't clip (B-AMP-1).
inside.rs:4: y /* trailing */
EOF
check "nothing staged lists nothing" </dev/null

# 2. The doc comment above a changed function is listed.
sed -i 's/x \* 2.0/x + x/' doc.rs
git add doc.rs
check "a changed function lists its doc comment" <<'EOF'
== doc.rs
-- fn double(x: f32) -> f32 (lines 1-5)
doc.rs:1: /// Doubles `x`.
EOF
commit "2"

# 3. A comment edited in the change isn't listed; the other one is.
sed -i -e 's/3 dB/6 dB/' -e 's/1.41/2.0/' edited.rs
git add edited.rs
check "a comment the change edited is not listed" <<'EOF'
== edited.rs
-- fn limit(x: f32) -> f32 (lines 1-6)
edited.rs:4: // Unchanged.
EOF
commit "3"

# 4. Outside every item: a `use` with no comment lists nothing; a const lists
#    its doc comment; a line inside a multi-line const lists that statement's
#    comments, the `[f32; 3]` not ending it. The `//!` header isn't listed,
#    whether an inner attribute or nothing is between it and the statement.
sed -i -e 's/consts::PI/consts::TAU/' -e 's/0.89/0.9/' -e 's/    3.0,/    4.0,/' outside.rs
sed -i 's/std::fmt/std::io/' header.rs
git add outside.rs header.rs
check "changes outside any item list their statement's comments" <<'EOF'
== outside.rs
-- const CEILING: f32 = 0.9; (lines 5-6)
outside.rs:5: /// The ceiling, -1 dBFS (B-LIM-2).
-- const TABLE: [f32; 3] = [ (lines 8-13)
outside.rs:8: // A table, its entries commented.
outside.rs:10: 1.0, // unity
outside.rs:11: 2.0, // double
EOF
commit "4"

# 5. Nested items: a change in helper lists helper's comments only, the one
#    directly above it being its doc comment; one in f lists f's, without
#    helper's; one between the methods lists the impl's
#    own, without the methods'; a method added whole lists nothing. A pure
#    deletion inside h lists h's.
sed -i 's/x + 1/x + 2/' nested.rs
git add nested.rs
check "a change in a nested fn lists only its comments" <<'EOF'
== nested.rs
-- fn helper(x: usize) -> usize (lines 13-17)
nested.rs:13: // In f, before the nested fn.
nested.rs:15: // In helper.
EOF
commit "5a"
sed -i 's/helper(Self::N)/helper(Self::N) + 1/' nested.rs
git add nested.rs
check "a change in the outer fn lists its own comments, not the nested fn's" <<'EOF'
== nested.rs
-- fn f(&self) -> usize (lines 11-20)
nested.rs:11: /// f's doc.
nested.rs:18: // In f, after.
EOF
commit "5b"
sed -i 's/const N: usize = 4;/const N: usize = 8;/' nested.rs
git add nested.rs
check "a change between methods lists the impl's own comments" <<'EOF'
== nested.rs
-- impl S (lines 6-30)
nested.rs:6: /// The impl's doc.
nested.rs:8: // Between methods.
EOF
commit "5c"
python3 -I - <<'PY'
import pathlib
p = pathlib.Path('nested.rs')
p.write_text(p.read_text().replace(
    "        /// g's doc.\n",
    "        /// k's doc, new.\n        fn k(&self) {\n            // New too.\n        }\n\n        /// g's doc.\n"))
PY
git add nested.rs
check "a method added whole lists nothing" </dev/null
commit "5d"
sed -i '/let _ = 1;/d' nested.rs
git add nested.rs
check "a deletion lists the item around it" <<'EOF'
== nested.rs
-- fn h(&self) (lines 30-33)
nested.rs:30: /// h's doc.
nested.rs:32: // In h.
EOF
commit "5e"

# 6. A deleted file lists nothing; so do a deletion between items and one
#    between file-level statements.
git rm -q gone.rs
sed -i '/use std::io;/d' shrink.rs
sed -i '/^fn shrink() {$/,/^}$/d' shrink.rs
git add shrink.rs
check "a deleted file or item lists nothing" </dev/null
commit "6"

# 7. A new file lists nothing: every line in it is new. Markdown isn't read.
cat >new.rs <<'EOF'
/// New.
fn new() {
    // New too.
}
EOF
echo "// still not Rust" >>notes.md
git add new.rs notes.md
check "a new file lists nothing" </dev/null
commit "7"

# 8. Literals and comments that look like braces or comments don't confuse
#    the matching: the doc comment, both block comment lines and the real
#    comment are listed, `after`'s doc isn't.
sed -i 's/+ xs.len()/+ xs.len() + 1/' lexing.rs
git add lexing.rs
check "literals and nested block comments are read as Rust reads them" <<'EOF'
== lexing.rs
-- fn tricky<'a>(s: &'a str, xs: [u8; 2]) -> usize (lines 1-12)
lexing.rs:1: /// Literals that look like comments or braces.
lexing.rs:7: /* a block /* nested */ comment
lexing.rs:8: over two lines */
lexing.rs:10: // The real comment.
EOF
commit "8"

# 9. Code mid-edit, a stray `}` and a block comment never closed, still gives
#    a list.
cat >broken.rs <<'EOF'
fn broken() {
    // Comment.
    let x = 1;
}
}
/* never closed
EOF
commit "9"
sed -i 's/let x = 1;/let x = 2;/' broken.rs
git add broken.rs
check "unfinished code still gives a list" <<'EOF'
== broken.rs
-- fn broken() (lines 1-4)
broken.rs:2: // Comment.
EOF
commit "9b"

# 10. Wrong use: too many arguments, an unknown base.
check "too many arguments" a b c <<'EOF'
usage: scripts/comment-drift.sh [BASE [HEAD]]
(exit 1)
EOF
check "an unknown base" no-such-base <<'EOF'
fatal: bad revision 'no-such-base...HEAD'
(exit 128)
EOF

if [[ $failed -ne 0 ]]; then
    echo "test-comment-drift: FAILED"
    exit 1
fi
echo "test-comment-drift: all passed"
