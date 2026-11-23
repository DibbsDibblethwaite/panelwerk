#!/usr/bin/env bash
# The consumer check (CLAUDE.md rule 14): every consumer built against this
# checkout and its full hook run. A tag is cut only when every consumer
# passes. Adapted from DSPWerkbank's scripts/consumers.sh (its D-1).
#
# Each consumer is checked as committed (its current branch's HEAD), in a
# clone of its own under target/consumers/, so its working tree is never
# touched. The clone takes this checkout through a [patch] entry in
# target/consumers/.cargo/config.toml, a directory above the clone, which
# cargo reads as it reads any parent directory's config, and `cargo update`
# of the library's crates moves the clone's Cargo.lock onto it (a lock
# pinning another version would leave the patch unused).
#
# The moved lock is then committed in the clone, a local commit never
# pushed anywhere, so the clone's working tree is clean when the consumer's
# hook runs: Rödalog's and Klinga's hooks each refuse a working tree that
# differs from its index. Committed, not staged (W-7's suggestion, tried
# 2026-11-23): staged, it does set off Kör's, Fläns's and Klinga's licence
# checks, but each of those refuses, by design, a Panelwerk not taken by
# tag ("not in the build once, by tag"), so in a clone patched onto this
# checkout they can only fail. A licence a Panelwerk change could alter (a
# new dependency's) is therefore checked at the family update, by each
# consumer's own hook on the tagged lock, not here. Nothing else in the
# clone changes, and the next run resets it to the consumer's HEAD. The
# clones keep their builds between runs.
#
# With names (`./scripts/consumers.sh Klinga`), only those consumers are
# checked, and the last line says so: a tag needs the run with none.
#
#   --pins  only report each consumer's pinned tag, and fail unless every
#           consumer's pin is read and all are the same one (CLAUDE.md,
#           Family update); a pin the script can't read fails, named (W-8).
#
# Exits non-zero if any consumer fails, naming each that did; 2 for a name
# that is no consumer's.
set -uo pipefail
cd "$(dirname "$0")/.."
root=$PWD
work="$root/target/consumers"

# The consumers: name|checkout|its checks, run in the clone's root. Each one
# is listed as it adopts Panelwerk (docs/proposal.md, step 1). Each one's
# checks are its full hook, which runs its panel's tests and its value-text
# tests. Rödalog's hook keeps its passes in a cache outside the repository
# (its scripts/hook-skip.sh: ~/.cache/roedalog, which its deploy gate
# reads), so the clone is given a cache of its own under target/consumers/:
# the check never writes Rödalog's records. Its hook then decides, as it
# always does, whether its engine's tests can be skipped (the engine's
# inputs as in a run that passed in this cache; the engine doesn't use
# Panelwerk), and says so in one line.
consumers=(
    "Rödalog|$HOME/code/roedalog|ROEDALOG_HOOK_CACHE='$work/roedalog-hook-cache' ./scripts/hooks/pre-commit"
    "Kör|$HOME/code/koer|./scripts/hooks/pre-commit"
    "Fläns|$HOME/code/flaens|./scripts/hooks/pre-commit"
    "Klinga|$HOME/code/klinga|./scripts/hooks/pre-commit"
)

# The git source the consumers name the library by.
source_url="https://github.com/DibbsDibblethwaite/panelwerk"

# Whether the first argument equals one of the rest, as a fixed string (not
# a pattern: `K.*` must name no consumer, not select Kör and Klinga).
is_one_of() {
    local want=$1 candidate
    shift
    for candidate in "$@"; do
        [[ "$candidate" == "$want" ]] && return 0
    done
    return 1
}

if [[ "${1:-}" == --pins ]]; then
    if (($# > 1)); then
        echo "consumers: --pins takes no names: a family update moves every consumer" >&2
        exit 2
    fi
    tags=() unread=()
    for entry in "${consumers[@]}"; do
        IFS='|' read -r name checkout _ <<<"$entry"
        tag=$(grep -oE "panelwerk\"[^}]*tag = \"v[0-9.]+\"" "$checkout/Cargo.toml" | grep -oE 'v[0-9.]+' | sort -u | tr '\n' ' ')
        echo "consumers: $name pins ${tag:-nothing this script can read}"
        if [[ -z "$tag" ]]; then
            unread+=("$name")
        else
            tags+=("$tag")
        fi
    done
    # A pin not read (a `rev =`, a `.git` URL, a table over several lines)
    # fails, named: before W-8 it counted as a tag "none", so consumers
    # that all read as none were "every pin the same".
    if ((${#unread[@]})); then
        echo "consumers: no pin read for ${unread[*]}: read its Cargo.toml by hand, or teach this script its form"
        exit 1
    fi
    if [[ $(printf '%s\n' "${tags[@]}" | sort -u | wc -l) -ne 1 ]]; then
        echo "consumers: the pins differ: a family update is due"
        exit 1
    fi
    echo "consumers: every consumer pins the same"
    exit 0
fi

# Names given on the command line: only those consumers, each checked to be
# one of the list's, so a misspelt name fails rather than checks nothing.
only=("$@")
names=()
for entry in "${consumers[@]}"; do names+=("${entry%%|*}"); done
for name in "${only[@]}"; do
    if ! is_one_of "$name" "${names[@]}"; then
        echo "consumers: no consumer named $name" >&2
        exit 2
    fi
done

mkdir -p "$work/.cargo"
cat >"$work/.cargo/config.toml" <<CONFIG
# Written by scripts/consumers.sh: the consumers' clones below take this
# checkout in place of the tagged library.
[patch."$source_url"]
panelwerk-text = { path = "$root/crates/panelwerk-text" }
panelwerk-panel = { path = "$root/crates/panelwerk-panel" }
panelwerk-testkit = { path = "$root/crates/panelwerk-testkit" }
panelwerk-nice = { path = "$root/crates/panelwerk-nice" }
CONFIG

failed=()
for entry in "${consumers[@]}"; do
    IFS='|' read -r name checkout checks <<<"$entry"
    if ((${#only[@]})) && ! is_one_of "$name" "${only[@]}"; then
        continue
    fi
    start=$SECONDS
    echo "consumers: $name ($checkout)"
    clone="$work/$(basename "$checkout")"
    if [[ ! -d "$clone/.git" ]]; then
        git clone --quiet --no-hardlinks "$checkout" "$clone" || { failed+=("$name (clone)"); continue; }
    fi
    # The consumer's HEAD, exactly: the clone's own changes (the lock's
    # local commit, any file left) undone, its build kept.
    if ! { git -C "$clone" fetch --quiet "$checkout" HEAD &&
        git -C "$clone" checkout --quiet --force --detach FETCH_HEAD &&
        git -C "$clone" clean --quiet -fd; }; then
        failed+=("$name (update)")
        continue
    fi
    # A consumer takes one crate or several, so each is updated if its lock
    # has it.
    crates=()
    for c in panelwerk-text panelwerk-panel panelwerk-testkit panelwerk-nice; do
        grep -q "^name = \"$c\"" "$clone/Cargo.lock" && crates+=("$c")
    done
    if ((${#crates[@]} == 0)) || ! (cd "$clone" && cargo update --quiet "${crates[@]}"); then
        failed+=("$name (no Panelwerk in its lock, or the update failed)")
        continue
    fi
    # Proof the patch was in force: the library resolved from this checkout,
    # not from git (cargo only warns of an unused patch). A path dependency
    # is the one kind a lock writes without a `source =` line, so any
    # library package (a name starting "panelwerk") with one, from whatever
    # URL or registry, means the patch didn't take.
    if awk '/^\[\[package\]\]/ { lib = 0 }
        /^name = "panelwerk/ { lib = 1 }
        lib && /^source = / { found = 1 }
        END { exit !found }' "$clone/Cargo.lock"; then
        echo "consumers: $name: the library still resolved from a source (git or a registry), not this checkout"
        failed+=("$name (patch not applied)")
        continue
    fi
    echo "consumers: $name at $(git -C "$clone" log -1 --format='%h %s')"
    # The moved lock committed in the clone (the header says why). The
    # identity is the clone's own, set here so the commit never depends on
    # the machine's git config; it is never pushed. A lock that didn't move
    # (no change to commit) is refused: the patch check above would have
    # caught a library still from git, so this is a lock already on a path.
    if git -C "$clone" diff --quiet -- Cargo.lock ||
        ! git -C "$clone" -c user.name=consumers.sh -c user.email=consumers.sh@localhost \
            -c commit.gpgsign=false commit --quiet \
            --message "consumers.sh: the library taken from $root" -- Cargo.lock; then
        failed+=("$name (commit the lock)")
        continue
    fi
    if (cd "$clone" && bash -c "$checks"); then
        echo "consumers: $name passed in $(( (SECONDS - start) / 60 ))m$(( (SECONDS - start) % 60 ))s"
    else
        echo "consumers: $name FAILED after $(( (SECONDS - start) / 60 ))m$(( (SECONDS - start) % 60 ))s"
        failed+=("$name")
    fi
done

if ((${#failed[@]})); then
    echo "consumers: FAILED: ${failed[*]}"
    exit 1
fi
if ((${#only[@]})); then
    echo "consumers: all passed of those named (${only[*]}); a tag needs every consumer"
else
    echo "consumers: all passed"
fi
