#!/bin/sh
set -eu
directory=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
test -x "$directory/localscribe"
applications="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
mkdir -p "$applications"
# The Exec field of a desktop entry treats percent signs as field codes.
executable=$(printf '%s' "$directory/localscribe" | sed 's/[%]/%%/g; s/[\\"$\x60]/\\\\&/g')
entry="$applications/localscribe.desktop"
cat > "$entry" <<ENTRY
[Desktop Entry]
Version=1.0
Type=Application
Name=LocalScribe
Comment=Private audio and video transcription
Exec="$executable"
Terminal=false
Categories=AudioVideo;Utility;
ENTRY
printf 'LocalScribe added to the applications menu.\n'
