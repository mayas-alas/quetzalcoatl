# Runbook de build del instalador Quetzalcoatl

Este procedimiento prepara un host Windows, valida el workspace y genera el
MSI y el bundle reproducible de Quetzalcoatl 0.1.4.

Debe ejecutarse desde una copia confiable del repositorio. No introduce
secretos, no instala el producto y no publica artefactos.

## 1. Resultado esperado

El build genera:

```text
target\installer\Quetzalcoatl.msi
target\installer\QuetzalcoatlSetup.exe
```

El gate de empaquetado exige dos builds consecutivos desde el mismo árbol de
fuentes. El MSI y el EXE del primer build deben ser idénticos byte por byte a
los del segundo.

## 2. Requisitos del host

### Sistema

- Windows 11 x64.
- PowerShell 5.1 o posterior.
- Acceso HTTPS a GitHub, crates.io y NuGet.
- Al menos 10 GiB libres recomendados para herramientas, cachés y artefactos.
- Una cuenta que pueda autorizar la instalación de componentes de desarrollo.

WSL, Podman y Tailscale no necesitan estar instalados para hacer el build. El
script descarga los instaladores y la imagen fijados, verifica sus tamaños y
SHA-256 y los incorpora al bundle.

### Cadena de herramientas

- Rust 1.96.1 para `x86_64-pc-windows-msvc`.
- Componentes Rust `rustfmt` y `clippy`.
- Visual Studio Build Tools 2022:
  - workload `Microsoft.VisualStudio.Workload.VCTools`;
  - compilador y linker MSVC x64;
  - Windows SDK.
- .NET SDK 8.
- Git para Windows, incluido Git Bash.
- `curl.exe`, incluido en Windows 11.

WiX no se instala globalmente. El manifiesto `.config\dotnet-tools.json`
restaura WiX Toolset 5.0.2 de forma local.

## 3. Abrir PowerShell en el repositorio

```powershell
Set-Location -LiteralPath 'C:\Users\mayas\quetzalcoatl'
```

Confirme que se encuentra en la raíz:

```powershell
Get-Item -LiteralPath '.\Cargo.toml', '.\installer\build.ps1'
```

## 4. Instalar las herramientas

Los siguientes comandos usan `winget`. Windows puede solicitar elevación.

### .NET SDK 8

```powershell
winget install `
  --id Microsoft.DotNet.SDK.8 `
  --exact `
  --source winget `
  --accept-package-agreements `
  --accept-source-agreements `
  --silent `
  --disable-interactivity
```

### Rustup

```powershell
winget install `
  --id Rustlang.Rustup `
  --exact `
  --source winget `
  --accept-package-agreements `
  --accept-source-agreements `
  --silent `
  --disable-interactivity
```

### Git para Windows

```powershell
winget install `
  --id Git.Git `
  --exact `
  --source winget `
  --accept-package-agreements `
  --accept-source-agreements `
  --silent `
  --disable-interactivity
```

### Visual Studio Build Tools 2022

```powershell
winget install `
  --id Microsoft.VisualStudio.2022.BuildTools `
  --exact `
  --source winget `
  --accept-package-agreements `
  --accept-source-agreements `
  --silent `
  --disable-interactivity `
  --override '--wait --quiet --norestart --nocache --installPath C:\BuildTools --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended'
```

Cuando termine la instalación, cierre y vuelva a abrir PowerShell para que
Windows refresque `PATH`.

## 5. Instalar la toolchain Rust fijada

Desde la raíz del repositorio:

```powershell
rustup toolchain install 1.96.1 --profile minimal
rustup target add x86_64-pc-windows-msvc --toolchain 1.96.1
rustup component add `
  rustfmt `
  clippy `
  --toolchain 1.96.1-x86_64-pc-windows-msvc
```

`rust-toolchain.toml` selecciona automáticamente Rust 1.96.1 al trabajar
dentro del repositorio.

## 6. Preparar el entorno MSVC

Cada PowerShell nuevo debe importar el entorno de Visual Studio antes de
compilar. Este bloque supone que Build Tools se instaló en `C:\BuildTools`:

```powershell
$env:Path = @(
  "$env:USERPROFILE\.cargo\bin"
  'C:\Program Files\dotnet'
  'C:\Program Files\Git\cmd'
  'C:\Program Files\Git\bin'
  $env:Path
) -join ';'

$vsDev = 'C:\BuildTools\Common7\Tools\VsDevCmd.bat'

if (-not (Test-Path -LiteralPath $vsDev)) {
  throw "No se encontró Visual Studio Build Tools: $vsDev"
}

$devEnvironment = cmd.exe /d /s /c `
  "`"`"$vsDev`" -arch=x64 -host_arch=x64 >nul && set`""

foreach ($line in $devEnvironment) {
  if ($line -match '^([^=]+)=(.*)$') {
    Set-Item -Path "Env:$($matches[1])" -Value $matches[2]
  }
}
```

Si Build Tools se instaló en otra ubicación, localice `VsDevCmd.bat` y ajuste
`$vsDev`.

## 7. Verificar versiones y herramientas

```powershell
rustup show active-toolchain
rustc --version
cargo --version
dotnet --version
git --version
curl.exe --version

Get-Command `
  rustc, `
  cargo, `
  dotnet, `
  git, `
  curl.exe, `
  cl.exe, `
  link.exe
```

Valores esperados para las herramientas fijadas por el repositorio:

```text
rustc 1.96.1
cargo 1.96.1
WiX Toolset 5.0.2
```

La versión exacta de .NET 8, Git, MSVC y Windows SDK puede avanzar dentro de
su línea compatible.

## 8. Validar el workspace

Ejecute primero los checks más estrechos:

```powershell
cargo fmt --all -- --check

if ($LASTEXITCODE -ne 0) {
  throw 'cargo fmt falló.'
}

cargo clippy -p gnx-service -- -D warnings

if ($LASTEXITCODE -ne 0) {
  throw 'cargo clippy falló.'
}

cargo test --workspace

if ($LASTEXITCODE -ne 0) {
  throw 'cargo test falló.'
}
```

No continúe al empaquetado si alguno falla.

## 9. Primer build

```powershell
powershell.exe `
  -NoProfile `
  -ExecutionPolicy Bypass `
  -File installer\build.ps1

if ($LASTEXITCODE -ne 0) {
  throw 'El primer build del instalador falló.'
}
```

El script:

1. valida la identidad congelada de la versión 0.1.4;
2. restaura WiX 5.0.2 y la extensión determinista;
3. descarga los artefactos fijados;
4. verifica tamaño y SHA-256 de cada descarga;
5. compila los tres ejecutables Rust con CRT estático;
6. construye el MSI y el bundle Burn;
7. normaliza sus identidades y timestamps;
8. extrae el bundle y verifica su registro y payload embebido.

## 10. Conservar el primer resultado

Guarde la primera pareja fuera de `target\installer` antes del segundo build:

```powershell
$firstBuild = Join-Path $PWD 'target\repro-first'
New-Item -ItemType Directory -Force -Path $firstBuild | Out-Null

Copy-Item `
  -LiteralPath 'target\installer\Quetzalcoatl.msi' `
  -Destination (Join-Path $firstBuild 'Quetzalcoatl.msi') `
  -Force

Copy-Item `
  -LiteralPath 'target\installer\QuetzalcoatlSetup.exe' `
  -Destination (Join-Path $firstBuild 'QuetzalcoatlSetup.exe') `
  -Force

Get-FileHash `
  -Algorithm SHA256 `
  (Join-Path $firstBuild 'Quetzalcoatl.msi'), `
  (Join-Path $firstBuild 'QuetzalcoatlSetup.exe')
```

No cambie fuentes, toolchains ni archivos fijados entre ambos builds.

## 11. Segundo build

```powershell
powershell.exe `
  -NoProfile `
  -ExecutionPolicy Bypass `
  -File installer\build.ps1

if ($LASTEXITCODE -ne 0) {
  throw 'El segundo build del instalador falló.'
}
```

## 12. Probar reproducibilidad byte por byte

```powershell
$comparisons = @(
  @{
    Name = 'Quetzalcoatl.msi'
    First = 'target\repro-first\Quetzalcoatl.msi'
    Second = 'target\installer\Quetzalcoatl.msi'
  }
  @{
    Name = 'QuetzalcoatlSetup.exe'
    First = 'target\repro-first\QuetzalcoatlSetup.exe'
    Second = 'target\installer\QuetzalcoatlSetup.exe'
  }
)

$results = foreach ($comparison in $comparisons) {
  $first = Get-Item -LiteralPath $comparison.First
  $second = Get-Item -LiteralPath $comparison.Second
  $firstHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $first.FullName).Hash
  $secondHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $second.FullName).Hash

  [pscustomobject]@{
    Artifact = $comparison.Name
    FirstBytes = $first.Length
    SecondBytes = $second.Length
    FirstSHA256 = $firstHash
    SecondSHA256 = $secondHash
    ByteIdentical = (
      $first.Length -eq $second.Length -and
      $firstHash -eq $secondHash
    )
  }
}

$results | Format-List

if ($results.ByteIdentical -contains $false) {
  throw 'El build no es reproducible byte por byte.'
}
```

Un resultado aceptable muestra `ByteIdentical : True` para ambos artefactos.

## 13. Validar el payload de clúster

Git para Windows proporciona `sh.exe`:

```powershell
$sh = 'C:\Program Files\Git\bin\sh.exe'
$clusterScript = 'runtime/payload-v1/bin/gnx-pve-cluster-create'

& $sh -n $clusterScript

if ($LASTEXITCODE -ne 0) {
  throw 'La validación de sintaxis shell falló.'
}

& $sh $clusterScript static-check

if ($LASTEXITCODE -ne 0) {
  throw 'El static-check del payload falló.'
}
```

Compruebe que el SHA-256 del script coincide con el manifiesto:

```powershell
$actualHash = (
  Get-FileHash `
    -Algorithm SHA256 `
    -LiteralPath 'runtime\payload-v1\bin\gnx-pve-cluster-create'
).Hash.ToLowerInvariant()

$manifest = Get-Content `
  -LiteralPath 'runtime\payload-v1\manifest.json' `
  -Raw |
  ConvertFrom-Json

$manifestEntry = $manifest.files |
  Where-Object {
    $_.source -eq 'bin/gnx-pve-cluster-create' -or
    $_.path -eq 'bin/gnx-pve-cluster-create' -or
    $_.destination -like '*gnx-pve-cluster-create'
  } |
  Select-Object -First 1

if (-not $manifestEntry) {
  throw 'No se encontró gnx-pve-cluster-create en el manifiesto.'
}

if ($manifestEntry.sha256 -ne $actualHash) {
  throw 'El hash del payload no coincide con manifest.json.'
}

"Payload SHA-256 verificado: $actualHash"
```

## 14. Inspeccionar los artefactos finales

```powershell
$artifacts = @(
  'target\installer\Quetzalcoatl.msi'
  'target\installer\QuetzalcoatlSetup.exe'
)

$artifactReport = foreach ($path in $artifacts) {
  $file = Get-Item -LiteralPath $path

  [pscustomobject]@{
    Path = $file.FullName
    Bytes = $file.Length
    MiB = [math]::Round($file.Length / 1MB, 2)
    SHA256 = (
      Get-FileHash -Algorithm SHA256 -LiteralPath $file.FullName
    ).Hash
  }
}

$artifactReport | Format-List
```

Para la fuente validada el 23 de julio de 2026, los resultados fueron:

```text
Quetzalcoatl.msi
Bytes: 269582336
SHA-256: 82D030A7147F9CA3DE36E5D4AB8681909F67E6500F1A70DE01F7201FC1AD3834

QuetzalcoatlSetup.exe
Bytes: 557536137
SHA-256: 9A447111E213EC7C1FCB93668A2E2A2F43A3511218E55BE03C2C0AF4F998A510
```

Estos hashes identifican únicamente esa fuente concreta. Si el árbol de
fuentes cambia, registre los hashes nuevos y no mezcle evidencia entre
candidatos.

## 15. Comprobar que no se modificaron fuentes

```powershell
git status --short
git diff --check
```

Los archivos de `target` están ignorados. Un build limpio no debe modificar
archivos versionados.

## 16. Criterio de éxito

El build se considera correcto cuando:

- formato, Clippy y todas las pruebas pasan;
- el script oficial termina con exit code 0;
- WiX no reporta errores;
- el MSI y el EXE existen;
- el payload pasa `sh -n` y `static-check`;
- el hash del payload coincide con `manifest.json`;
- dos builds consecutivos producen tamaños y SHA-256 idénticos;
- `git diff --check` no reporta errores.

Esto demuestra compilación y empaquetado. No demuestra instalación,
reinicio/reanudación, WSL2, Podman, KVM, Tailscale, Proxmox ni quorum.

## 17. Fallos frecuentes

### `cargo-fmt.exe is not installed`

```powershell
rustup component add `
  rustfmt `
  --toolchain 1.96.1-x86_64-pc-windows-msvc
```

### `cargo-clippy.exe is not installed`

```powershell
rustup component add `
  clippy `
  --toolchain 1.96.1-x86_64-pc-windows-msvc
```

### No se encuentra `link.exe` o `cl.exe`

Vuelva a ejecutar la sección **Preparar el entorno MSVC** en la terminal
actual. Confirme:

```powershell
Get-Command cl.exe, link.exe
```

### No se encuentra `dotnet`, `cargo` o `git`

Cierre y abra PowerShell después de instalar las herramientas. Si es
necesario, aplique el bloque de `PATH` de la sección de entorno MSVC.

### Falla una descarga bloqueada

Compruebe acceso HTTPS a los orígenes:

```powershell
Invoke-WebRequest `
  -Method Head `
  -Uri 'https://api.nuget.org/v3/index.json' `
  -UseBasicParsing

Invoke-WebRequest `
  -Method Head `
  -Uri 'https://github.com' `
  -UseBasicParsing
```

No sustituya manualmente una dependencia. El build debe aceptar únicamente
los archivos cuyos tamaños y hashes coinciden con
`installer\dependencies.lock.json`.

### Los dos builds no coinciden

No publique el resultado. Confirme que:

- no cambió ningún archivo entre builds;
- ambos builds usaron Rust 1.96.1 y WiX 5.0.2;
- se ejecutaron desde el mismo checkout;
- no se sustituyeron artefactos en `target\installer-cache`;
- el primer resultado se copió antes de ejecutar el segundo build.

Conserve ambos pares y sus hashes para diagnóstico, sin alterar la identidad
congelada del candidato.

## 18. Límites operativos

- No ejecute el instalador en el host de build salvo que esa prueba esté
  planeada explícitamente.
- No publique, firme, suba ni distribuya artefactos sin autorización.
- No incluya secretos, claves Tailscale, passwords PVE ni blobs DPAPI en
  comandos, logs o evidencia.
- El build actual genera artefactos sin firma digital.
- La aceptación del clúster exige tres hosts físicos Windows 11 y se rige por
  `docs\VALIDATION.md`; este documento no reemplaza ese gate.
