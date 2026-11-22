#!/bin/sh
set -eu
directory=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
test -x "$directory/localscribe"
applications="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
mkdir -p "$applications"
# Path is a desktop-entry string, so directory characters never enter the Exec command.
entry_path=$(printf '%s\n' "$directory" | awk '
    NR > 1 { printf "\\n" }
    {
        for (index_char = 1; index_char <= length($0); index_char++) {
            character = substr($0, index_char, 1)
            if (character == "\\") printf "\\\\"
            else if (character == " ") printf "\\s"
            else if (character == "\t") printf "\\t"
            else if (character == "\r") printf "\\r"
            else printf "%s", character
        }
    }
')
entry="$applications/localscribe.desktop"
cat > "$entry" <<ENTRY
[Desktop Entry]
Version=1.0
Type=Application
Name=LocalScribe
Comment=Private audio and video transcription
TryExec=$entry_path/localscribe
Path=$entry_path
Exec=/bin/sh -c "exec ./localscribe"
Terminal=false
Categories=AudioVideo;Audio;
ENTRY
printf 'LocalScribe added to the applications menu.\n'
