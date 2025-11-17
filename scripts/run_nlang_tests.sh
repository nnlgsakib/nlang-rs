#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
test_dir_default="$repo_root/nlang_examples"
test_dir="${1:-$test_dir_default}"
cd "$test_dir"

mapfile -t nlang_files < <(find "$test_dir" -maxdepth 1 -type f -name '*.nlang' | sort)
passed=()
failed=()

echo "=== Running nlang Tests ==="

for file in "${nlang_files[@]}"; do
  base="${file%.nlang}"
  exe_win="${base}.exe"
  exe_unix="${base}"
  echo "Compiling $(basename "$file")..."
  if cargo run --bin nlang --manifest-path "$repo_root/Cargo.toml" -- compile "$file"; then
    echo "Compiled successfully: $(basename "$file")"
    run_target=""
    if [[ -f "$exe_win" ]]; then run_target="$exe_win"; fi
    if [[ -z "$run_target" && -f "$exe_unix" ]]; then run_target="$exe_unix"; fi
    if [[ -n "$run_target" ]]; then
      echo "Running $(basename "$run_target")..."
      if "$run_target"; then
        passed+=("$(basename "$file")")
      else
        failed+=("$(basename "$file")")
      fi
      rm -f "$exe_win" "$exe_unix" || true
    else
      echo "No executable found for $(basename "$file")"
      failed+=("$(basename "$file")")
    fi
  else
    echo "Compilation failed for $(basename "$file")"
    failed+=("$(basename "$file")")
  fi
  echo "--------------------------------------------"
done

echo "=== Test Summary ==="
echo "Passed: ${#passed[@]}"
for p in "${passed[@]}"; do echo "  $p"; done
echo ""
echo "Failed: ${#failed[@]}"
for f in "${failed[@]}"; do echo "  $f"; done
echo ""
echo "=== All Done ==="