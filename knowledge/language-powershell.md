# PowerShell Conventions and Idioms

## PowerShell Philosophy

PowerShell is a cross-platform task automation shell and scripting language. Unlike Unix shells that pipe text, PowerShell pipes .NET objects. This makes structured data manipulation natural without text parsing.

- **Objects, not text**: Pipeline passes rich .NET objects with properties and methods.
- **Verb-Noun naming**: `Get-Process`, `Set-Content`, `New-Item` — consistent, discoverable commands.
- **Cross-platform**: PowerShell 7+ runs on Windows, macOS, and Linux.

## Pipeline and Object Manipulation

```powershell
# Pipeline passes objects, not text
Get-Process |
    Where-Object { $_.CPU -gt 100 } |
    Sort-Object CPU -Descending |
    Select-Object Name, CPU, WorkingSet |
    Format-Table -AutoSize

# Select specific properties
Get-Service | Select-Object Name, Status, StartType

# Filter with Where-Object
Get-ChildItem -Recurse -File |
    Where-Object { $_.Extension -eq '.log' -and $_.Length -gt 1MB }

# Transform with ForEach-Object
1..10 | ForEach-Object { $_ * $_ }

# Group and measure
Get-EventLog -LogName System -Newest 1000 |
    Group-Object -Property Source |
    Sort-Object Count -Descending |
    Select-Object -First 10
```

## Functions

```powershell
function Get-DiskReport {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory, ValueFromPipeline)]
        [string[]]$ComputerName,

        [ValidateSet('GB', 'MB', 'TB')]
        [string]$Unit = 'GB',

        [switch]$IncludeRemovable
    )

    process {
        foreach ($computer in $ComputerName) {
            Get-CimInstance -ClassName Win32_LogicalDisk -ComputerName $computer |
                Where-Object { $IncludeRemovable -or $_.DriveType -eq 3 } |
                Select-Object @{N='Computer'; E={$computer}},
                              DeviceID,
                              @{N="Size($Unit)"; E={[math]::Round($_.Size / "1$Unit", 2)}},
                              @{N="Free($Unit)"; E={[math]::Round($_.FreeSpace / "1$Unit", 2)}}
        }
    }
}

# Usage
'Server1', 'Server2' | Get-DiskReport -Unit GB
```

## Error Handling

```powershell
# Try/Catch/Finally
try {
    $content = Get-Content -Path $path -ErrorAction Stop
    $data = $content | ConvertFrom-Json
}
catch [System.IO.FileNotFoundException] {
    Write-Warning "File not found: $path"
}
catch [System.Management.Automation.RuntimeException] {
    Write-Warning "JSON parse error: $_"
}
catch {
    Write-Error "Unexpected error: $($_.Exception.Message)"
    throw  # re-throw
}
finally {
    # cleanup
}

# ErrorAction parameter
# Stop     — throw terminating error
# Continue — write error, keep going (default)
# SilentlyContinue — suppress error
# Inquire  — prompt user

# $ErrorActionPreference sets the default for the scope
$ErrorActionPreference = 'Stop'
```

## Working with Files and Data

```powershell
# File operations
$content = Get-Content -Path 'config.json' -Raw | ConvertFrom-Json
$content.setting = 'new_value'
$content | ConvertTo-Json -Depth 10 | Set-Content -Path 'config.json'

# CSV
$data = Import-Csv -Path 'users.csv'
$data | Where-Object { $_.Department -eq 'Engineering' } |
    Export-Csv -Path 'engineers.csv' -NoTypeInformation

# REST APIs
$response = Invoke-RestMethod -Uri 'https://api.example.com/users' -Headers @{
    Authorization = "Bearer $token"
}
$response.data | ForEach-Object { $_.name }

# Regex
$text = 'Error: Code 404 at 2024-03-15'
if ($text -match 'Code (\d+) at (\d{4}-\d{2}-\d{2})') {
    $code = $Matches[1]  # "404"
    $date = $Matches[2]  # "2024-03-15"
}
```

## Modules

```powershell
# Module manifest (MyModule.psd1)
@{
    ModuleVersion     = '1.0.0'
    RootModule        = 'MyModule.psm1'
    FunctionsToExport = @('Get-Widget', 'Set-Widget', 'Remove-Widget')
    Author            = 'Your Name'
    Description       = 'Widget management module'
}

# Module script (MyModule.psm1)
function Get-Widget {
    [CmdletBinding()]
    param(
        [string]$Name = '*'
    )
    # implementation
}

function Set-Widget {
    [CmdletBinding(SupportsShouldProcess)]
    param(
        [Parameter(Mandatory)]
        [string]$Name,
        [hashtable]$Properties
    )
    if ($PSCmdlet.ShouldProcess($Name, 'Update widget')) {
        # implementation
    }
}
```

## Classes (PowerShell 5+)

```powershell
class Logger {
    [string]$Path
    [ValidateSet('Info', 'Warning', 'Error')]
    [string]$Level = 'Info'

    Logger([string]$path) {
        $this.Path = $path
    }

    [void] Log([string]$message) {
        $entry = "[{0}] [{1}] {2}" -f (Get-Date -Format 'o'), $this.Level, $message
        Add-Content -Path $this.Path -Value $entry
    }

    [void] Error([string]$message) {
        $this.Level = 'Error'
        $this.Log($message)
        $this.Level = 'Info'
    }
}
```

## Conventions

1. **Use `[CmdletBinding()]`** on all functions. It adds `-Verbose`, `-Debug`, `-ErrorAction`, and other common parameters.
2. **Use `-ErrorAction Stop`** in `try` blocks. PowerShell non-terminating errors don't trigger `catch` by default.
3. **Verb-Noun naming** for all exported functions. Run `Get-Verb` to see approved verbs.
4. **Pipeline-friendly functions.** Accept `ValueFromPipeline` input and process in the `process {}` block.
5. **Use `ShouldProcess`** for destructive operations. Enables `-WhatIf` and `-Confirm`.
6. **Avoid `Write-Host`** for data output. Use `Write-Output` (or just return values). `Write-Host` bypasses the pipeline.

---

_Sources: PowerShell Documentation (Microsoft), PowerShell in Action (Bruce Payette), The PowerShell Best Practices and Style Guide_
