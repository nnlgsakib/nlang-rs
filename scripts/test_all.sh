#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
test_dir_default="$repo_root/nlang_examples"
test_dir="${1:-$test_dir_default}"
cd "$test_dir"

mapfile -t files < <(find "$test_dir" -maxdepth 1 -type f -name '*.nlang' | grep -E '/[0-9]+_.+\.nlang$' | sort)
passed=()
failed=()

echo "=== Running nlang Tests ==="

for file in "${files[@]}"; do
  echo "Testing $(basename "$file")..."
  if cargo run --bin nlang --manifest-path "$repo_root/Cargo.toml" -- run "$file" > /dev/null 2>&1; then
    echo "Passed: $(basename "$file")"
    passed+=("$(basename "$file")")
  else
    echo "Failed: $(basename "$file")"
    failed+=("$(basename "$file")")
  fi
done

echo ""
echo "=== Test Summary ==="
echo "Passed: ${#passed[@]}"
for p in "${passed[@]}"; do echo "  $p"; done
echo ""
echo "Failed: ${#failed[@]}"
for f in "${failed[@]}"; do echo "  $f"; done
echo ""
echo "=== All Done ==="