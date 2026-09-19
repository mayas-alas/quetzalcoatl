#requires -Version 5.1
<##
.SYNOPSIS
  Retired compatibility shim for the old dynamic Ubuntu provisioning path.

.DESCRIPTION
  GNX never downloads or exports an Ubuntu distribution at install time.
  Runtime provisioning must use the authenticated rootfs.tar supplied through
  install.ps1 and gnx-setup.exe. This command fails closed so legacy callers
  cannot silently create an unauthenticated runtime.
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
[ordered]@{
    result = 'BLOCKED'
    code = 'DYNAMIC_ROOTFS_PROVISIONING_REMOVED'
    next_action = 'Use install.ps1 with the sealed manifest and authenticated rootfs.tar.'
} | ConvertTo-Json -Compress | Write-Output
exit 1
