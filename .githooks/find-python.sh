# Sourced by the hooks. Prints the first interpreter that actually runs Python 3.
# `command -v python3` alone is not enough: on Windows it can resolve to the Microsoft Store stub,
# which exists but cannot run anything.
find_python() {
  for candidate in python3 python py; do
    if command -v "$candidate" >/dev/null 2>&1 && "$candidate" -c "import sys; sys.exit(0 if sys.version_info[0] == 3 else 1)" >/dev/null 2>&1; then
      echo "$candidate"
      return 0
    fi
  done
  return 1
}
