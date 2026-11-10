#!/usr/bin/env bash
# The consumer check (CLAUDE.md rule 14): every consumer built against this
# checkout and its full checks run. A tag is cut only when every consumer
# passes. Adapted from DSPWerkbank's scripts/consumers.sh.
#
# Each consumer is checked as committed (its current branch's HEAD), in a
# clone of its own under target/consumers/, so its working tree is never
# touched. The clone takes this checkout through a [patch] entry in
# target/consumers/.cargo/config.toml, a directory above the clone, which
# cargo reads as it reads any parent directory's config, and `cargo update`
# of the library's crates moves the clone's Cargo.lock onto it (a lock
# pinning another version would leave the patch unused). Nothing else in
# the clone changes, and the lock is reset at the next run. The clones keep
# their builds between runs.
#
#   --pins  only report each consumer's pinned tag, and fail unless all
#           three pin the same one (CLAUDE.md, Family update).
#
# Exits non-zero if any consumer fails, naming each that did.
set -uo pipefail
cd "$(dirname "$0")/.."
root=$PWD

# The consumers: name|checkout|its checks, run in the clone's root. Each one
# is listed as it adopts Panelwerk (docs/proposal.md, step 1); until then the
# entry stays commented out, since a consumer that doesn't use the library
# would fail the patch check below. Each one's checks are its full hook,
# which runs its panel's tests and its value-text tests.
consumers=(
    "Rödalog|$HOME/code/roedalog|./scripts/hooks/pre-commit"
    "Kör|$HOME/code/koer|./scripts/hooks/pre-commit"
    "Fläns|$HOME/code/flaens|./scripts/hooks/pre-commit"
)

# The git source the consumers name the library by.
source_url="https://github.com/DibbsDibblethwaite/panelwerk"

if ((${#consumers[@]} == 0)); then
    echo "consumers: none yet (each is listed as it adopts Panelwerk)"
    exit 0
fi

if [[ "${1:-}" == --pins ]]; then
    tags=()
    for entry in "${consumers[@]}"; do
        IFS='|' read -r name checkout _ <<<"$entry"
        tag=$(grep -oE "panelwerk\"[^}]*tag = \"v[0-9.]+\"" "$checkout/Cargo.toml" | grep -oE 'v[0-9.]+' | sort -u | tr '\n' ' ')
        echo "consumers: $name pins ${tag:-nothing}"
        tags+=("${tag:-none}")
    done
    if [[ $(printf '%s\n' "${tags[@]}" | sort -u | wc -l) -ne 1 ]]; then
        echo "consumers: the pins differ: a family update is due"
        exit 1
    fi
    echo "consumers: every pin the same"
    exit 0
fi

work="$root/target/consumers"
mkdir -p "$work/.cargo"
cat >"$work/.cargo/config.toml" <<CONFIG
# Written by scripts/consumers.sh: the consumers' clones below take this
# checkout in place of the tagged library.
[patch."$source_url"]
panelwerk-text = { path = "$root/crates/panelwerk-text" }
panelwerk-panel = { path = "$root/crates/panelwerk-panel" }
panelwerk-testkit = { path = "$root/crates/panelwerk-testkit" }
CONFIG

failed=()
for entry in "${consumers[@]}"; do
    IFS='|' read -r name checkout checks <<<"$entry"
    start=$SECONDS
    echo "consumers: $name ($checkout)"
    clone="$work/$(basename "$checkout")"
    if [[ ! -d "$clone/.git" ]]; then
        git clone --quiet --no-hardlinks "$checkout" "$clone" || { failed+=("$name (clone)"); continue; }
    fi
    # The consumer's HEAD, exactly: the clone's own changes (the
    # re-resolved Cargo.lock) undone, its build kept. A consumer takes
    # either crate or both, so each is updated if its lock has it.
    if ! { git -C "$clone" fetch --quiet "$checkout" HEAD &&
        git -C "$clone" checkout --quiet --force --detach FETCH_HEAD &&
        git -C "$clone" clean --quiet -fd; }; then
        failed+=("$name (update)")
        continue
    fi
    crates=()
    for c in panelwerk-text panelwerk-panel panelwerk-testkit; do
        grep -q "^name = \"$c\"" "$clone/Cargo.lock" && crates+=("$c")
    done
    if ((${#crates[@]} == 0)) || ! (cd "$clone" && cargo update --quiet "${crates[@]}"); then
        failed+=("$name (no Panelwerk in its lock, or the update failed)")
        continue
    fi
    # Proof the patch was in force: the library resolved from this checkout,
    # not from git (cargo only warns of an unused patch).
    if grep -q "^source = \"git+$source_url" "$clone/Cargo.lock"; then
        echo "consumers: $name: the library still resolved from git, not this checkout"
        failed+=("$name (patch not applied)")
        continue
    fi
    echo "consumers: $name at $(git -C "$clone" log -1 --format='%h %s')"
    if (cd "$clone" && bash -c "$checks"); then
        echo "consumers: $name passed in $(( (SECONDS - start) / 60 ))m$(( (SECONDS - start) % 60 ))s"
    else
        echo "consumers: $name FAILED"
        failed+=("$name")
    fi
done

if ((${#failed[@]})); then
    echo "consumers: FAILED: ${failed[*]}"
    exit 1
fi
echo "consumers: all passed"
