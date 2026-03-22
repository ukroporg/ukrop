pub fn init_script() -> &'static str {
    r#"
# ukrop shell integration for PowerShell

# Hook: record directory on every prompt
$global:__ukrop_hooked = $null
$global:__ukrop_last_cmd = $null

function global:__ukrop_hook {
    $cwd = (Get-Location).ProviderPath
    if ($null -ne $cwd) {
        & ukrop hook -- $cwd
    }
}

# Initialize prompt hook
if ($global:__ukrop_hooked -ne 1) {
    $global:__ukrop_hooked = 1
    $global:__ukrop_prompt_old = $function:prompt

    function global:prompt {
        $lastExit = $LASTEXITCODE

        # Record last command
        $histItem = Get-History -Count 1
        if ($null -ne $histItem -and $histItem.Id -ne $global:__ukrop_last_hist_id) {
            $global:__ukrop_last_hist_id = $histItem.Id
            $cmd = $histItem.CommandLine
            $cwd = (Get-Location).ProviderPath
            $durationMs = [int]$histItem.Duration.TotalMilliseconds
            & ukrop hook-cmd --cmd $cmd --exit-code $lastExit --cwd $cwd --duration-ms $durationMs 2>$null
        }

        __ukrop_hook

        $global:LASTEXITCODE = $lastExit
        if ($null -ne $__ukrop_prompt_old) {
            & $__ukrop_prompt_old
        } else {
            "PS $($executionContext.SessionState.Path.CurrentLocation)$('>' * ($nestedPromptLevel + 1)) "
        }
    }
}

function global:ukrop {
    if ($args.Count -eq 0 -or $args[0] -eq 'cd' -or $args[0] -eq 'run' -or $args[0] -eq 'ssh') {
        $encoding = [Console]::OutputEncoding
        try {
            [Console]::OutputEncoding = [System.Text.Utf8Encoding]::new()
            $result = & (Get-Command ukrop -CommandType Application | Select-Object -First 1) @args 2>$null
        } finally {
            [Console]::OutputEncoding = $encoding
        }
        if ($LASTEXITCODE -ne 0) { return }
        if ([string]::IsNullOrEmpty($result)) { return }

        # Shift+Enter: put text on command line for editing
        if ($result.StartsWith('edit:')) {
            $result = $result.Substring(5)
            switch -Regex ($result) {
                '^cd:(.*)' {
                    $path = $Matches[1]
                    [Microsoft.PowerShell.PSConsoleReadLine]::Insert("Set-Location -LiteralPath '$path'")
                }
                '^run:(.*)' {
                    [Microsoft.PowerShell.PSConsoleReadLine]::Insert($Matches[1])
                }
                '^ssh:(.*)' {
                    [Microsoft.PowerShell.PSConsoleReadLine]::Insert("ssh $($Matches[1])")
                }
            }
            return
        }

        switch -Regex ($result) {
            '^cd:(.*)' {
                $path = $Matches[1]
                Set-Location -LiteralPath $path
            }
            '^run:(.*)' {
                $cmd = $Matches[1]
                & ukrop hook-cmd --cmd $cmd --exit-code 0 --cwd (Get-Location).ProviderPath 2>$null
                Invoke-Expression $cmd
            }
            '^ssh:(.*)' {
                $sshArgs = $Matches[1]
                & ukrop hook-ssh --host $sshArgs 2>$null
                Invoke-Expression "ssh $sshArgs"
            }
        }
    } else {
        & (Get-Command ukrop -CommandType Application | Select-Object -First 1) @args
    }
}

# Ctrl+R binding for command history (requires PSReadLine)
if (Get-Module -Name PSReadLine -ErrorAction Ignore) {
    Set-PSReadLineKeyHandler -Chord 'Ctrl+r' -ScriptBlock {
        $encoding = [Console]::OutputEncoding
        try {
            [Console]::OutputEncoding = [System.Text.Utf8Encoding]::new()
            $result = & (Get-Command ukrop -CommandType Application | Select-Object -First 1) run 2>$null
        } finally {
            [Console]::OutputEncoding = $encoding
        }
        if ($LASTEXITCODE -ne 0) { return }
        if ([string]::IsNullOrEmpty($result)) {
            [Microsoft.PowerShell.PSConsoleReadLine]::InvokePrompt()
            return
        }

        $edit = $false
        if ($result.StartsWith('edit:')) {
            $result = $result.Substring(5)
            $edit = $true
        }

        $line = switch -Regex ($result) {
            '^cd:(.*)' { "Set-Location -LiteralPath '$($Matches[1])'" }
            '^run:(.*)' { $Matches[1] }
            '^ssh:(.*)' { "ssh $($Matches[1])" }
        }

        [Microsoft.PowerShell.PSConsoleReadLine]::RevertLine()
        [Microsoft.PowerShell.PSConsoleReadLine]::Insert($line)
        if (-not $edit) {
            [Microsoft.PowerShell.PSConsoleReadLine]::AcceptLine()
        }
    }
}

Set-Alias -Name u -Value ukrop -Option AllScope -Scope Global -Force
"#
}
