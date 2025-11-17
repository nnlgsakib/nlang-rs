# Run all .nlang files in the current directory with nlang compiler
# and delete generated .exe files after execution

# Directory containing the .nlang test files
$testDir = "D:\projects\rust\nlang\nlang_examples"
Set-Location $testDir

# Collect test files
$nlangFiles = Get-ChildItem -Path $testDir -Filter "*.nlang"
$passed = @()
$failed = @()

Write-Host "=== Running nlang Tests ===`n"

foreach ($file in $nlangFiles) {
    $exePath = [System.IO.Path]::ChangeExtension($file.FullName, ".exe")

    Write-Host "Compiling $($file.Name)..."

    # Run the compiler
    $compile = & cargo run --bin nlang --manifest-path "D:\projects\rust\nlang\Cargo.toml" -- compile $file.FullName 2>&1

    if ($LASTEXITCODE -eq 0) {
        Write-Host "✅ Compiled successfully: $($file.Name)"
        
        # Run the generated exe if it exists
        if (Test-Path $exePath) {
            try {
                Write-Host "▶ Running $([System.IO.Path]::GetFileName($exePath))..."
                & $exePath
                if ($LASTEXITCODE -eq 0) {
                    $passed += $file.Name
                } else {
                    $failed += $file.Name
                }
            } catch {
                Write-Host "❌ Runtime error in $($file.Name): $_"
                $failed += $file.Name
            }
            # Clean up
            Remove-Item $exePath -ErrorAction SilentlyContinue
        } else {
            Write-Host "⚠ No executable found for $($file.Name)"
            $failed += $file.Name
        }
    } else {
        Write-Host "❌ Compilation failed for $($file.Name)"
        Write-Host $compile
        $failed += $file.Name
    }

    Write-Host "`n--------------------------------------------`n"
}

# Summary
Write-Host "=== Test Summary ==="
Write-Host "✅ Passed: $($passed.Count)"
foreach ($p in $passed) { Write-Host "   $p" }
Write-Host "`n❌ Failed: $($failed.Count)"
foreach ($f in $failed) { Write-Host "   $f" }

Write-Host "`n=== All Done ==="
