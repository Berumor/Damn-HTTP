#!/usr/bin/env bash
# Checks every non-merge commit in BASE..HEAD:
#   1. the subject is a Conventional Commit (the release tooling reads each one, D-023);
#   2. the commit is signed off by its author (DCO).
# Usage: check-commits.sh <base> <head>
set -euo pipefail

base="$1"
head="$2"

types='feat|fix|docs|test|chore|refactor|ci|build|perf|revert'
subject_re="^(${types})(\([a-z0-9][a-z0-9._/-]*\))?!?: [^ ].*$"

failed=0
count=0

while read -r sha; do
  count=$((count + 1))
  short="${sha:0:7}"
  subject="$(git log -1 --format=%s "$sha")"
  author_email="$(git log -1 --format=%ae "$sha")"
  signoffs="$(git log -1 --format='%(trailers:key=Signed-off-by,valueonly)' "$sha")"

  if ! [[ "$subject" =~ $subject_re ]]; then
    echo "::error::${short}: subject is not a Conventional Commit: \"${subject}\""
    echo "  Expected: <type>[(scope)][!]: <description>, with type one of: ${types//|/, }"
    failed=1
  fi

  if ! grep -qiF "<${author_email}>" <<<"$signoffs"; then
    echo "::error::${short}: missing \"Signed-off-by\" line for the author <${author_email}>"
    echo "  Fix with: git commit --amend -s (last commit) or git rebase --signoff (whole branch)"
    failed=1
  fi
done < <(git rev-list --no-merges "${base}..${head}")

if [ "$failed" -ne 0 ]; then
  echo "See CONTRIBUTING.md for the commit rules."
  exit 1
fi

echo "Checked ${count} commit(s): all are Conventional Commits and signed off."
