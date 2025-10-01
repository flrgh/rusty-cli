#!/usr/bin/env bash

set -euo pipefail

readonly WORKFLOW=.github/workflows/release.yml
readonly CONFIG=dist-workspace.toml

mapfile -t ACTIONS < <(
  yq -r < "$WORKFLOW" '
    [
      .jobs[].steps[].uses
      | select(.)
      | select(test("@"))
    ]
    | unique_by(.)
    | sort_by(.)
    | .[]'
)

LF=$'\n'
TEXT='[dist.github-action-commits]'

for action in "${ACTIONS[@]}"; do
  name=${action%@*}
  sha=${action#*@}
  echo "action: $name, sha: $sha"
  TEXT="${TEXT}${LF}\"${name}\" = \"${sha}\""
done


# add
if ! grep -qxF '# dependabot BEGIN' "$CONFIG"; then
  printf '\n%s\n%s\n%s\n' \
    '# dependabot BEGIN' \
    "$TEXT" \
    '# dependabot END' \
  >> "$CONFIG"

# update
else
  TMP=$(mktemp)

  awk '
    BEGIN {
      do_print = 1
    }

    /^# dependabot BEGIN$/ {
      do_print = 0;
      print "# dependabot BEGIN";
      print TEXT;
    }

    /^# dependabot END$/ {
      do_print = 1
    }

    {
      if (do_print) { print }
    }

  ' TEXT="$TEXT" "$CONFIG" \
  > "$TMP"

  cat "$TMP" > "$CONFIG"
  rm -f "$TMP" || true
fi

git diff "$CONFIG" || true
