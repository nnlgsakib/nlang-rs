# Compile all .ll files in current directory to .exe
$files = Get-ChildItem -Filter *.ll

foreach ($file in $files) {
    $out = [System.IO.Path]::ChangeExtension($file.FullName, ".exe")

    Write-Host "Compiling $($file.Name) -> $(Split-Path $out -Leaf)" -ForegroundColor Cyan

    # Run clang
    $result = clang "$($file.FullName)" -o "$out" 2>&1

    if ($LASTEXITCODE -eq 0) {
        Write-Host "✅ Success: $($file.Name)" -ForegroundColor Green
    } else {
        Write-Host "❌ Failed: $($file.Name)" -ForegroundColor Red
        Write-Host "---- clang output ----"
        Write-Host $result
        Write-Host "----------------------"
    }
}
