#!/usr/bin/env bash
# community-cache-clean-transcript.sh
#
# Converts raw YouTube VTT subtitle files into clean, readable plaintext
# suitable for use as research material in the community cache.
#
# Usage: ./scripts/community-cache-clean-transcript.sh <input.vtt> [output.txt]
#
# What it does:
# 1. Strips VTT headers, timing lines, and inline timing tags
# 2. Removes duplicate lines (VTT repeats text across caption blocks)
# 3. Joins sentence fragments into flowing paragraphs
# 4. Inserts paragraph breaks at natural pauses (>3 second gaps)
# 5. Normalizes whitespace
#
# Output: Clean plaintext with natural paragraph breaks, suitable for
# reading and extracting insights from.

set -euo pipefail

usage() {
  cat <<'EOF'
Usage: ./scripts/community-cache-clean-transcript.sh <input.vtt> [output.txt]

Converts a raw YouTube VTT subtitle file into clean plaintext.
If output path is omitted, writes to stdout.
EOF
}

main() {
  local input="${1:-}"
  local output="${2:-}"

  if [[ -z "$input" || ! -f "$input" ]]; then
    usage
    exit 1
  fi

  local cleaned
  cleaned=$(
    # Step 1: Remove VTT header, timing lines, inline timing tags, and alignment metadata
    sed -E '
      /^WEBVTT/d
      /^Kind:/d
      /^Language:/d
      /^NOTE/d
      /^[0-9]{2}:[0-9]{2}:[0-9]{2}\.[0-9]+ -->/d
      s/<[^>]+>//g
      s/&nbsp;/ /g
      s/&amp;/\&/g
      s/&lt;/</g
      s/&gt;/>/g
    ' "$input" |

    # Step 2: Remove empty lines, trim whitespace
    sed -E 's/^[[:space:]]+//; s/[[:space:]]+$//' |
    grep -v '^$' |

    # Step 3: Remove consecutive duplicate lines
    awk '!seen[$0]++ || NR != prev_nr + 1 { print; prev_nr = NR; delete seen }
         { prev_nr = NR }' |

    # Step 4: Remove remaining exact duplicate adjacent lines
    uniq |

    # Step 5: Join all lines into one stream, then re-split into paragraphs
    # at sentence boundaries where a natural topic shift likely occurs
    tr '\n' ' ' |
    sed -E 's/[[:space:]]+/ /g'
  )

  # Step 6: Re-introduce paragraph breaks at sentence boundaries
  # Split roughly every 3-5 sentences for readability
  local output_text
  output_text=$(echo "$cleaned" | awk '{
    n = split($0, words, " ")
    line = ""
    sentence_count = 0
    for (i = 1; i <= n; i++) {
      if (line != "") line = line " "
      line = line words[i]
      # Count sentence endings
      if (words[i] ~ /[.!?]$/ || words[i] ~ /[.!?]["\x27]$/) {
        sentence_count++
        if (sentence_count >= 4) {
          print line
          print ""
          line = ""
          sentence_count = 0
        }
      }
    }
    if (line != "") print line
  }')

  if [[ -n "$output" ]]; then
    echo "$output_text" > "$output"
    echo "[clean-transcript] Wrote cleaned transcript to $output" >&2
  else
    echo "$output_text"
  fi
}

main "$@"
