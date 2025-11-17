# test_all.ps1
# Run all .nlang test files that start with numbers (like 01_, 02_...) and check for errors.

$testDir = ".\nlang_examples"

# Get files starting with digits and underscore
$files = Get-ChildItem $testDir -Filter "*.nlang" | Where-Object { $_.Name -match "^\d+_" }

$results = @()

foreach ($file in $files) {
    Write-Host "Testing $($file.Name)..."
    try {
        cargo run --bin nlang -- run $file.FullName | Out-Null
        if ($LASTEXITCODE -eq 0) {
            Write-Host "Passed: $($file.Name)" -ForegroundColor Green
            $results += [pscustomobject]@{ File = $file.Name; Status = "Passed" }
        } else {
            Write-Host "Failed: $($file.Name)" -ForegroundColor Red
            $results += [pscustomobject]@{ File = $file.Name; Status = "Failed" }
        }
    }
    catch {
        Write-Host "Error running: $($file.Name)" -ForegroundColor Red
        $results += [pscustomobject]@{ File = $file.Name; Status = "Error" }
    }
}

# Summary
Write-Host "`n=== Test Summary ==="
$results | Format-Table -AutoSize
