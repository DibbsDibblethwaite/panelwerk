#!/usr/bin/env bash
# The comments to re-read after a change (CLAUDE.md, Guards against comment
# drift, 1; Ian, 2026-11-11, COMMENT-DRIFT). A comment that no longer matches
# its code is a defect, and the comments most likely to drift are the ones a
# change didn't touch in the code it did. This lists them, for people to read;
# it never judges them, and the hooks print it for information only.
#
#   scripts/comment-drift.sh               the staged changes (the hooks' use)
#   scripts/comment-drift.sh BASE [HEAD]   the changes on HEAD since its
#                                          merge-base with BASE (a reviewer's
#                                          use: `v3`, `main`, a commit); HEAD
#                                          is the current commit by default
#
# Output, grouped by file and item, nothing when there's nothing to list
# (and, if the list can't be made, the error and a line saying so, on
# stderr, and a non-zero exit):
#   == path/to/file.rs
#   -- pub fn process(&mut self) (lines 12-40)
#   path/to/file.rs:14: // the comment line, as it stands
#
# The rules (one file at a time, Rust source only, `*.rs`):
# - An item is a `{ … }` block whose header (the code since the previous `;`,
#   `{` or `}`) holds one of fn, impl, trait, struct, enum, union, mod or
#   macro_rules!: fn and union only before a name (not a `fn(…)` type, nor a
#   `.union(` call or a variable), and none as a macro's `$trait` or a raw
#   `r#struct` (DRIFT-3). It runs from its doc comment (the comment lines directly above its header and
#   attributes, up to a blank line) to its closing brace.
# - Each changed line belongs to the innermost item that holds it. Listed for
#   that item: every line in it with comment text (`//`, `///`, `//!`,
#   `/* … */`, trailing comments after code too) that the change didn't
#   itself add or edit, except those inside items nested in it, which have
#   their own lists. So a change in a method lists the method's comments and
#   doc comment, not the impl's; a change in the impl between its methods
#   lists the impl's own comments and doc comment, not the methods'.
# - A changed line outside every item (a `use`, `const`, `static` or `type` at
#   file level) lists the statement it's in, to its `;`, with its doc comment.
#   Inner doc comments (`//!`, `/*!`) and inner attributes (`#![…]`) are the
#   file's or module's, never the doc comment of what follows them, so the
#   file's header isn't listed for a change below it.
# - A hunk that only deletes lines touches the innermost item holding the
#   lines on both sides of the gap (or the file-level statement holding both).
# - Changed lines that are blank or hold only closing brackets (`}`, `});`)
#   are ignored, so an item added whole lists nothing
#   (every line of it is new) and doesn't count as touching the item around it.
#   A new file lists nothing; a deleted file lists nothing.
#
# Limits, accepted for a list that's simple and predictable:
# - It reads tokens, not Rust's grammar. if, match, loop and closure blocks
#   aren't items, so a change inside one lists its whole function's comments.
#   A `const` or `static` with a braced initialiser, and `extern` blocks, are
#   statements, not items.
# - Strings, raw strings, char literals and nested block comments are skipped
#   when matching braces; a char literal is told from a lifetime by its
#   closing quote (`'{'` is a char, `'a` a lifetime).
# - Only the new side is read: a comment that the change deleted can't be
#   listed, nor a comment elsewhere (another item, another file, the docs)
#   that describes the changed code.
# - With BASE, only committed changes count, not the working tree.
# - File names that git quotes (holding `"`, `\` or control characters)
#   aren't supported.
#
# Tests: scripts/test-comment-drift.sh (not run by the hook).
# The same script is copied, unchanged, into each of Ian's Rust projects.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
# If the list can't be made (a fault here, or git's), say so in plain words
# after the error, so that no list never reads as nothing to re-read
# (DRIFT-2: an IndexError went unnoticed in DSPWerkbank for two commits).
status=0
python3 -I - "$@" <<'PY' || status=$?
import bisect
import re
import subprocess
import sys

# An item's keyword (DRIFT-3). `fn` and `union` only before a name (one
# that begins with any letter, Unicode's too, or `_`, a raw `r#name` among
# them, its `r` a letter; or a macro's `$name`), so not a `fn(…) -> …`
# type in a closure's parameter or a cast, nor a `.union(` call; `union`
# is a keyword only there, and not
# before `as`, `in` or `else`, where it is a variable (`union as u8`, `for
# union in …`, `let … = union else`). None after `$` or `#`: a macro's
# metavariable (`$trait:ident`) and a raw identifier (`r#struct`) are
# names. impl, trait, struct, enum and mod are reserved words: outside a
# raw identifier, nothing else is spelled so.
ITEM_KEYWORD = re.compile(r'''
    (?<![$\#])\b(?: fn\s+(?:[^\W\d]|\$)
                | union\s+(?!(?:as|in|else)\b)(?:[^\W\d]|\$)
                | (?:impl|trait|struct|enum|mod)\b
                | macro_rules! )
''', re.X)
# What isn't code: comments and literals, found left to right.
LITERAL = re.compile(r'''
    (?P<line>//[^\n]*)
  | (?P<block>/\*)
  | (?<!\w)[bc]?r(?P<hashes>\#*)"
  | (?P<str>(?<!\w)[bc]?")
  | (?P<chr>(?<!\w)b?'(?:\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]{1,6}\}|.)|[^\\'\n])')
''', re.X)
BLOCK_EDGE = re.compile(r'/\*|\*/')
STRING_REST = re.compile(r'(?:[^"\\]|\\.)*"', re.S)
INNER_ATTRIBUTE = re.compile(r'#!\[[^\]]*\]')
NON_SPACE = re.compile(r'\S')
# A changed line of nothing but closing brackets carries no meaning of its own:
# git may show a neighbour's `}` as changed when a new item is added beside it.
EMPTY = re.compile(r'[\s})\];,]*')
HUNK = re.compile(r'@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@')


def git(args, stdin=None):
    done = subprocess.run(['git', '-c', 'core.quotePath=false', *args], input=stdin,
                          capture_output=True)
    if done.returncode != 0:
        sys.stderr.write(done.stderr.decode('utf-8', 'replace'))
        sys.exit(done.returncode)
    return done.stdout


def mask(text):
    """The text with comments blanked to spaces and literals to quotes (newlines
    kept, so positions and lines still match), and the comments' spans."""
    out, comments, pos = [], [], 0
    while (m := LITERAL.search(text, pos)):
        out.append(text[pos:m.start()])
        end = m.end()
        if m.group('block') is not None:
            depth = 1
            while depth:
                edge = BLOCK_EDGE.search(text, end)
                if not edge:
                    end = len(text)
                    break
                depth += 1 if edge.group() == '/*' else -1
                end = edge.end()
        elif m.group('hashes') is not None:
            close = text.find('"' + m.group('hashes'), end)
            end = len(text) if close < 0 else close + 1 + len(m.group('hashes'))
        elif m.group('str') is not None:
            rest = STRING_REST.match(text, end)
            end = rest.end() if rest else len(text)
        is_comment = m.group('line') is not None or m.group('block') is not None
        if is_comment:
            comments.append((m.start(), end))
        out.append(re.sub(r'[^\n]', ' ' if is_comment else '"', text[m.start():end]))
        pos = end
    out.append(text[pos:])
    # Its line breaks kept, as for comments: an attribute over several lines
    # blanked whole read every line below it a few lines off, and stopped
    # the script on an item near the file's end (DRIFT-2).
    masked = INNER_ATTRIBUTE.sub(lambda m: re.sub(r'[^\n]', ' ', m.group()), ''.join(out))
    return masked, comments


class Unit:
    """An item, or a file-level statement: lines first..last (1-based)."""
    def __init__(self, first, last, label):
        self.first, self.last, self.label = first, last, label


def analyse(text):
    lines = text.split('\n')
    masked, comment_spans = mask(text)
    masked_lines = masked.split('\n')
    starts = [0] + [m.end() for m in re.finditer('\n', text)]

    def line_of(pos):
        return bisect.bisect_right(starts, pos)

    has_code = [False] + [bool(m.strip()) for m in masked_lines]
    has_comment = [False] * (len(lines) + 1)
    for start, end in comment_spans:
        for line in range(line_of(start), line_of(max(start, end - 1)) + 1):
            has_comment[line] = True

    def first_code(lo, hi):
        """Position of the first code character in masked[lo:hi], or hi."""
        m = NON_SPACE.search(masked, lo, hi)
        return m.start() if m else hi

    def doc_start(line):
        while (line > 1 and has_comment[line - 1] and not has_code[line - 1]
               and not lines[line - 2].lstrip().startswith(('//!', '/*!'))):
            line -= 1
        return line

    items, statements = [], []
    # A `;` inside brackets (`[f32; 4]`) ends no statement: `nest` counts the
    # brackets open since the innermost `{`.
    stack, boundary, boundary0, nest = [], -1, -1, 0
    for m in re.finditer(r'[{};()\[\]]', masked):
        pos, ch = m.start(), m.group()
        if ch in '([':
            nest += 1
            continue
        if ch in ')]':
            nest = max(nest - 1, 0)
            continue
        if ch == ';' and nest:
            continue
        if ch == '{':
            stack.append((pos, first_code(boundary + 1, pos), nest))
            nest = 0
        elif ch == '}' and stack:
            open_pos, header, nest = stack.pop()
            keyword = ITEM_KEYWORD.search(masked, header, open_pos)
            if keyword:
                label = lines[line_of(keyword.start()) - 1].strip().rstrip('{').rstrip()
                items.append(Unit(doc_start(line_of(header)), line_of(pos), label))
        if not stack and ch != '{':
            start = first_code(boundary0 + 1, pos)
            statements.append(Unit(doc_start(line_of(start)), line_of(pos),
                                   lines[line_of(start) - 1].strip()))
            boundary0 = pos
        boundary = pos

    # Each line's innermost item: paint the largest first, the nested over them.
    owner = [None] * (len(lines) + 1)
    for item in sorted(items, key=lambda u: u.first - u.last):
        for line in range(item.first, item.last + 1):
            owner[line] = item
    statement_of = [None] * (len(lines) + 1)
    for statement in statements:
        for line in range(statement.first, statement.last + 1):
            if owner[line] is None:
                statement_of[line] = statement
    return lines, has_comment, items, owner, statement_of


def touched_units(text, added, gaps):
    lines, has_comment, items, owner, statement_of = analyse(text)
    count = len(lines) - (1 if text.endswith('\n') else 0)
    touched = {}
    for line in added:
        if line <= count and not EMPTY.fullmatch(lines[line - 1]):
            unit = owner[line] or statement_of[line]
            if unit:
                touched[id(unit)] = unit
    for after in gaps:
        around = [line for line in (after, after + 1) if 1 <= line <= count]
        holding = [i for i in items if around and i.first <= around[0] and around[-1] <= i.last]
        if holding:
            unit = min(holding, key=lambda u: u.last - u.first)
        elif around and statement_of[around[0]] is statement_of[around[-1]]:
            unit = statement_of[around[0]]
        else:
            unit = None
        if unit:
            touched[id(unit)] = unit
    result = []
    for unit in sorted(touched.values(), key=lambda u: u.first):
        listed = [(line, lines[line - 1].strip()) for line in range(unit.first, unit.last + 1)
                  if has_comment[line] and line not in added
                  and (owner[line] is unit or statement_of[line] is unit)]
        if listed:
            result.append((unit, listed))
    return result


def main(args):
    if len(args) > 2:
        sys.exit('usage: scripts/comment-drift.sh [BASE [HEAD]]')
    if args:
        head = args[1] if len(args) == 2 else 'HEAD'
        diff_args, rev = [f'{args[0]}...{head}'], head
    else:
        diff_args, rev = ['--cached'], ''
    diff = git(['diff', '--no-color', '--no-ext-diff', '--no-textconv', '-U0', '-M',
                '--src-prefix=a/', '--dst-prefix=b/',
                *diff_args, '--', '*.rs']).decode('utf-8', 'replace')
    changes, path, in_header = {}, None, False
    for line in diff.split('\n'):
        if line.startswith('diff --git '):
            path, in_header = None, True
        elif in_header and line.startswith('+++ '):
            name = line[4:].rstrip('\t')
            path = None if name == '/dev/null' else name[2:]
        elif line.startswith('@@') and path:
            in_header = False
            m = HUNK.match(line)
            start, count = int(m.group(1)), int(m.group(2) or 1)
            added, gaps = changes.setdefault(path, (set(), []))
            if count:
                added.update(range(start, start + count))
            else:
                gaps.append(start)
    if not changes:
        return
    paths = sorted(changes)
    blobs = git(['cat-file', '--batch'],
                ''.join(f'{rev}:{p}\n' for p in paths).encode('utf-8'))
    pos = 0
    for path in paths:
        newline = blobs.index(b'\n', pos)
        size = int(blobs[pos:newline].split()[2])
        text = blobs[newline + 1:newline + 1 + size].decode('utf-8', 'replace')
        pos = newline + 1 + size + 1
        found = touched_units(text, *changes[path])
        if found:
            print(f'== {path}')
        for unit, listed in found:
            print(f'-- {unit.label} (lines {unit.first}-{unit.last})')
            for line, comment in listed:
                print(f'{path}:{line}: {comment}')


main(sys.argv[1:])
PY
if ((status)); then
    echo "comment-drift: the list of comments to re-read couldn't be made (exit $status; the error is above)" >&2
    exit "$status"
fi
