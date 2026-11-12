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
cat >attribute.rs <<'EOF'
//! An inner attribute over several lines, as DSPWerkbank's springs spike
//! has (DRIFT-2).
#![allow(
    dead_code,
    unused_imports,
    unused_variables,
    clippy::all,
    clippy::pedantic
)]

/// One.
fn one() {}

/// Two.
fn two() -> u8 {
    // Two's comment.
    2
}
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

# 10. An inner attribute over several lines (DRIFT-2): blanking it took its
#     line breaks too, so every line below it was read a few lines off, and
#     an item in the file's last lines stopped the script (IndexError in
#     doc_start, on DSPWerkbank's springs spike). Lines below it are read
#     where they are: a change in the last item lists its doc and comment.
sed -i 's/^    2$/    3/' attribute.rs
git add attribute.rs
check "an inner attribute over several lines keeps the lines below it" <<'EOF'
== attribute.rs
-- fn two() -> u8 (lines 14-18)
attribute.rs:14: /// Two.
attribute.rs:16: // Two's comment.
EOF
commit "10"

# 11. Wrong use: too many arguments, an unknown base. Each failure ends with
#     a line saying plainly that there is no list (DRIFT-2), so a hook's
#     reader can't take it for nothing to re-read.
check "too many arguments" a b c <<'EOF'
usage: scripts/comment-drift.sh [BASE [HEAD]]
comment-drift: the list of comments to re-read couldn't be made (exit 1; the error is above)
(exit 1)
EOF
check "an unknown base" no-such-base <<'EOF'
fatal: bad revision 'no-such-base...HEAD'
comment-drift: the list of comments to re-read couldn't be made (exit 128; the error is above)
(exit 128)
EOF

# 12-15. Keywords that aren't items (DRIFT-3): a `fn` type in a closure's
#     parameter or a cast, and a `.union(` call, once made the block after
#     them a false item, which took the comment above it for its doc comment
#     and hid its comments from the function's list (simd_12a_acceptance.rs's
#     no-gain-steps test, 5b6f88b). `fn` and `union` count only before a
#     name; a real union is still an item.
cat >types.rs <<'EOF'
/// Applies a picker.
fn apply(xs: &[f32]) -> f32 {
    // The comment above the closure.
    let run = |f: fn(f32) -> f32| {
        // Inside the closure.
        f(1.0)
    };
    // After the closure.
    run(|x| x) + xs.len() as f32
}

/// Sums both sides.
fn sides(ps: &[(f32, f32)]) -> f32 {
    let mut total = 0.0;
    // The comment above the loop.
    for (name, g) in [("left", (|p| p.0) as fn(&(f32, f32)) -> f32), ("right", |p| p.1)] {
        // Inside the loop.
        total += ps.iter().map(g).sum::<f32>() + name.len() as f32;
    }
    // After the loop.
    total
}

/// The area a control covers.
fn area(c: &Control) -> Rect {
    // Columns cover their items.
    match c {
        Control::Column(items) => items.iter().fold(Rect::NOTHING, |r, c| r.union(area(c))),
        // A knob is its own rectangle.
        Control::Knob(k) => {
            // Inside the arm.
            k.rect
        }
    }
}

/// A float or its bits.
pub union Bits<T: Copy> {
    // The float.
    f: f32,
    u: T,
}
EOF
commit "12"
sed -i 's/xs.len() as f32$/xs.len() as f32 + 1.0/' types.rs
git add types.rs
check "a closure's fn-typed parameter makes no item" <<'EOF'
== types.rs
-- fn apply(xs: &[f32]) -> f32 (lines 1-10)
types.rs:1: /// Applies a picker.
types.rs:3: // The comment above the closure.
types.rs:5: // Inside the closure.
types.rs:8: // After the closure.
EOF
commit "12b"
sed -i 's/^    total$/    total * 2.0/' types.rs
git add types.rs
check "a cast to a fn type makes no item" <<'EOF'
== types.rs
-- fn sides(ps: &[(f32, f32)]) -> f32 (lines 12-22)
types.rs:12: /// Sums both sides.
types.rs:15: // The comment above the loop.
types.rs:17: // Inside the loop.
types.rs:20: // After the loop.
EOF
commit "13"
sed -i 's/^            k.rect$/            k.rect.clone()/' types.rs
git add types.rs
check "a .union( call makes no item" <<'EOF'
== types.rs
-- fn area(c: &Control) -> Rect (lines 24-35)
types.rs:24: /// The area a control covers.
types.rs:26: // Columns cover their items.
types.rs:29: // A knob is its own rectangle.
types.rs:31: // Inside the arm.
EOF
commit "14"
sed -i 's/^    u: T,$/    u: [T; 1],/' types.rs
git add types.rs
check "a union is still an item" <<'EOF'
== types.rs
-- pub union Bits<T: Copy> (lines 37-42)
types.rs:37: /// A float or its bits.
types.rs:39: // The float.
EOF
commit "15"

# 16-18. The same defect in other forms (DRIFT-3): a macro's metavariable
#     named after a keyword (`$trait`) made its arm a false item, and so did
#     a variable named `union` (a keyword only before a union's name);
#     a `fn $name` in a macro is still an item.
cat >macros.rs <<'EOF'
/// Adds its arguments.
macro_rules! add {
    // The one arm.
    ($a:expr, $trait:expr) => {
        // Inside the arm.
        $a + $trait
    };
}

/// Makes a getter.
macro_rules! getter {
    ($name:ident) => {
        /// The getter's doc.
        fn $name() -> u8 {
            // Inside the getter.
            1
        }
    };
}

/// Counts the unions.
fn count(unions: &[u8]) -> usize {
    let mut n = 0;
    // The comment above the loop.
    for union in unions {
        // Inside the loop.
        n += usize::from(*union);
    }
    n
}
EOF
commit "16"
sed -i 's/^        \$a + \$trait$/        $a + $trait + 0/' macros.rs
git add macros.rs
check "a macro's \$trait makes no item" <<'EOF'
== macros.rs
-- macro_rules! add (lines 1-8)
macros.rs:1: /// Adds its arguments.
macros.rs:3: // The one arm.
macros.rs:5: // Inside the arm.
EOF
commit "16b"
sed -i 's/^            1$/            2/' macros.rs
git add macros.rs
check "a macro's fn \$name is still an item" <<'EOF'
== macros.rs
-- fn $name() -> u8 (lines 13-17)
macros.rs:13: /// The getter's doc.
macros.rs:15: // Inside the getter.
EOF
commit "17"
sed -i 's/^    n$/    n + 0/' macros.rs
git add macros.rs
check "a variable named union makes no item" <<'EOF'
== macros.rs
-- fn count(unions: &[u8]) -> usize (lines 21-30)
macros.rs:21: /// Counts the unions.
macros.rs:24: // The comment above the loop.
macros.rs:26: // Inside the loop.
EOF
commit "18"

# 19-22. Names (DRIFT-3's review): a name may begin with a letter beyond
#     ASCII (`ébloui`) or be raw (`r#type`); `union as u8` and `union else`
#     are a variable, not a union; `r#struct` is a name, not the keyword.
cat >names.rs <<'EOF'
/// Dazzled.
fn ébloui() -> u8 {
    // Inside ébloui.
    1
}

/// A raw name.
fn r#type() -> u8 {
    // Inside type.
    1
}

/// Casts and raw names.
fn casts(union: u16, r#struct: u8) -> u8 {
    // Above the cast.
    if union as u8 > 3 {
        // Inside the cast's if.
        return 1;
    }
    // Above the raw name.
    if r#struct > 0 {
        // Inside the raw name's if.
        return 2;
    }
    0
}

/// A let-else.
fn unwrap(union: Option<u8>) -> u8 {
    // Above the let-else.
    let Some(v) = union else {
        // Inside the else.
        return 0;
    };
    v
}
EOF
commit "19"
sed -i '4s/1/2/' names.rs
git add names.rs
check "a name beginning with a letter beyond ASCII is an item" <<'EOF'
== names.rs
-- fn ébloui() -> u8 (lines 1-5)
names.rs:1: /// Dazzled.
names.rs:3: // Inside ébloui.
EOF
commit "19b"
sed -i '10s/1/2/' names.rs
git add names.rs
check "a raw name is an item" <<'EOF'
== names.rs
-- fn r#type() -> u8 (lines 7-11)
names.rs:7: /// A raw name.
names.rs:9: // Inside type.
EOF
commit "20"
sed -i '25s/0/3/' names.rs
git add names.rs
check "union as and r#struct make no item" <<'EOF'
== names.rs
-- fn casts(union: u16, r#struct: u8) -> u8 (lines 13-26)
names.rs:13: /// Casts and raw names.
names.rs:15: // Above the cast.
names.rs:17: // Inside the cast's if.
names.rs:20: // Above the raw name.
names.rs:22: // Inside the raw name's if.
EOF
commit "21"
sed -i '35s/v/v + 1/' names.rs
git add names.rs
check "union else makes no item" <<'EOF'
== names.rs
-- fn unwrap(union: Option<u8>) -> u8 (lines 28-36)
names.rs:28: /// A let-else.
names.rs:30: // Above the let-else.
names.rs:32: // Inside the else.
EOF
commit "22"

if [[ $failed -ne 0 ]]; then
    echo "test-comment-drift: FAILED"
    exit 1
fi
echo "test-comment-drift: all passed"
