if (Test-Path -Path ".\tmp\index-bare") {
    Write-Host "tmp\index-bare already exists, exiting"
    exit
}

New-Item -Path "." -Name "tmp" -ItemType "directory" -Force 2>&1 | out-null
Remove-Item -Path ".\tmp\index-bare" -Recurse -Force 2>&1 | out-null
Remove-Item -Path ".\tmp\index-co" -Recurse -Force 2>&1 | out-null

Write-Host "Initializing repository in tmp\index-bare..."
git init -q --bare ".\tmp\index-bare"

Write-Host "Creating checkout in tmp/index-bare..."
git init -q ".\tmp\index-co"
Set-Location ".\tmp\index-co"

New-Item -Path "." -Name "config.json" -ItemType "file" -Value @'
{
  "dl": "http://localhost:8888/api/v1/crates",
  "api": "http://localhost:8888/"
}
'@ 2>&1 | out-null
git add config.json 2>&1 | out-null
git commit -qm "Initial commit" 2>&1 | out-null
$origin = [string]::Format("file:///{0}/../index-bare", (Get-Location))
git remote add origin $origin 2>&1 | out-null
git push -q origin master -u 2>&1 | out-null
Set-Location ..\..
New-Item -Path "tmp\index-co\.git" -Name "git-daemon-export-ok" -ItemType "file" 2>&1 | out-null

Write-Host @'
Your local git index is ready to go!

Please refer to https://github.com/rust-lang/crates.io/blob/master/README.md for more info!
'@
