# ST55 premise P3: can PostgreSQL (in Docker Desktop) keep its data directory in a Windows folder?
# Prints only the shape of the result (OK / error kind / timings / row counts). No records, no paths beyond the test folder.
# Usage (Docker Desktop must be running):
#   powershell -ExecutionPolicy Bypass -File premise-pgdata.ps1
# It creates and removes the test folder %LOCALAPPDATA%\ashiato-pgtest, the volume ashiato-pgtest-vol
# and the containers ashiato-pgtest-*.
# Not measured: durability on power loss / forced stop (whether fsync reaches the Windows disk). fsync ops/sec is only a hint.
$ErrorActionPreference = "Continue"
$img = "postgres:17-alpine"
docker version --format "docker {{.Server.Version}} {{.Server.Os}}" 2>&1 | Select-Object -First 1

function Wait-Ready([string]$name) {
  # Ready = the real server accepts TCP (the init-time temporary server listens only on the unix socket).
  for ($i = 0; $i -lt 90; $i++) {
    Start-Sleep -Seconds 2
    docker exec $name pg_isready -h 127.0.0.1 -U postgres 2>&1 | Out-Null
    if ($LASTEXITCODE -eq 0) { return $true }
    $log = (docker logs $name 2>&1) -join "`n"
    if ($log -match "initdb: error|could not change permissions|wrong ownership|invalid permissions") { return $false }
    $state = docker inspect -f "{{.State.Running}}" $name 2>&1
    if ($state -ne "true") { return $false }
  }
  return $false
}

function Show-Failure([string]$label, [string]$name) {
  (docker logs $name 2>&1) | Select-String -Pattern "initdb: error|FATAL|could not|wrong ownership|invalid permissions|Operation not permitted" | Select-Object -First 3 | ForEach-Object { "[$label]   $($_.Line)" }
}

function Test-Mount([string]$label, [string]$mount) {
  $name = "ashiato-pgtest-$label"
  docker rm -f $name 2>$null | Out-Null
  docker run -d --name $name -e POSTGRES_PASSWORD=premise-only -v $mount $img | Out-Null
  if (-not (Wait-Ready $name)) {
    "[$label] init: FAILED"
    Show-Failure $label $name
    docker rm -f $name 2>$null | Out-Null
    return
  }
  "[$label] init: OK"
  docker exec $name psql -U postgres -q -c "create table t(a int)" 2>&1 | Out-Null
  "[$label] create table: rc=$LASTEXITCODE"
  $t = Measure-Command {
    docker exec $name psql -U postgres -q -c "do `$`$ begin for i in 1..2000 loop insert into t values (i); commit; end loop; end `$`$;" 2>&1 | Out-Null
  }
  $rc = $LASTEXITCODE
  "[$label] 2000 single-row commits: rc=$rc, {0:N1} s" -f $t.TotalSeconds
  docker exec $name sh -c "pg_test_fsync -s 2 -f /var/lib/postgresql/data/fsync.tmp 2>&1 | grep -E '^ +(fdatasync|fsync) ' | head -2"
  docker exec $name rm -f /var/lib/postgresql/data/fsync.tmp 2>$null | Out-Null

  # 1) restart the same container
  docker restart $name | Out-Null
  if (Wait-Ready $name) {
    $n = docker exec $name psql -U postgres -tA -c "select count(*) from t" 2>&1
    "[$label] rows after restart: $n"
  } else { "[$label] restart: FAILED"; Show-Failure $label $name }

  # 2) remove the container and attach the same data to a NEW container (like reinstalling Docker Desktop)
  docker rm -f $name 2>$null | Out-Null
  docker run -d --name $name -e POSTGRES_PASSWORD=premise-only -v $mount $img | Out-Null
  if (Wait-Ready $name) {
    $n = docker exec $name psql -U postgres -tA -c "select count(*) from t" 2>&1
    "[$label] rows in a new container: $n"
  } else { "[$label] new container: FAILED"; Show-Failure $label $name }
  docker rm -f $name 2>$null | Out-Null
}

$dir = Join-Path $env:LOCALAPPDATA "ashiato-pgtest"
New-Item -ItemType Directory -Force $dir | Out-Null
Test-Mount "winfolder" ("{0}:/var/lib/postgresql/data" -f $dir)
Test-Mount "volume" "ashiato-pgtest-vol:/var/lib/postgresql/data"
docker volume rm ashiato-pgtest-vol 2>$null | Out-Null
Remove-Item -Recurse -Force $dir -ErrorAction SilentlyContinue
"done"
