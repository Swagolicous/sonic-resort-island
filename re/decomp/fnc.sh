#!/bin/sh
# Print decompiled functions without local declarations (q.py fn + filter).
for a in "$@"; do
  python "$(dirname "$0")/q.py" fn "$a" | tail -n +2 | grep -vE '^\s+[A-Za-z_][A-Za-z0-9_ ]*[ \*]+[A-Za-z_][A-Za-z0-9_]*( \[[0-9]+\])?;$'
done
