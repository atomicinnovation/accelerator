#!/usr/bin/env bash
# Serve this prototype over HTTP so the browser can fetch the type="text/babel"
# .jsx sources — file:// blocks those under CORS.
set -euo pipefail

port="${1:-8000}"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
entry="Accelerator Visualiser.html"
url="http://localhost:${port}/$(printf '%s' "$entry" | sed 's/ /%20/g')"

echo "Serving ${root}"
echo "Open ${url}"

case "$(uname -s)" in
  Darwin) open "$url" ;;
  Linux) command -v xdg-open >/dev/null && xdg-open "$url" || true ;;
esac

exec python3 -m http.server "$port" --directory "$root"
