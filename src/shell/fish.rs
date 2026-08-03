pub fn init_script() -> &'static str {
    r#"
# ukrop shell integration for fish
set -g __ukrop_hook_err (mktemp -t ukrop-hook.XXXXXX)
set -g __ukrop_hook_warned 0

function __ukrop_hook --on-variable PWD
    command ukrop hook --shell-id "$fish_pid" -- "$PWD" 2>/dev/null
end

function __ukrop_postexec --on-event fish_postexec
    set -l exit_code $status
    command ukrop hook-cmd --cmd "$argv[1]" --exit-code "$exit_code" --cwd "$PWD" --duration-ms "$CMD_DURATION" 2>>$__ukrop_hook_err &
    if test "$__ukrop_hook_warned" = "0"; and test -s "$__ukrop_hook_err"
        set -g __ukrop_hook_warned 1
        echo "ukrop: command tracking error (see $__ukrop_hook_err):" >&2
        head -5 "$__ukrop_hook_err" >&2
    end
end

function ukrop
    if test (count $argv) -eq 0; or test "$argv[1]" = "cd"; or test "$argv[1]" = "run"; or test "$argv[1]" = "ssh"; or test "$argv[1]" = "search"
        set -l result (command ukrop $argv)
        or return $status
        if test -n "$result"
            # Shift+Enter: put text on command line for editing
            if string match -q 'edit:*' -- $result
                set result (string sub -s 6 -- $result)
                switch $result
                    case 'cd:*'
                        commandline -r "cd -- "(string sub -s 4 -- $result)
                    case 'run:*'
                        commandline -r (string sub -s 5 -- $result)
                    case 'ssh:*'
                        commandline -r "ssh "(string sub -s 5 -- $result)
                end
                commandline -f repaint
                return
            end
            switch $result
                case 'cd:*'
                    cd (string sub -s 4 -- $result)
                case 'run:*'
                    set -l cmd (string sub -s 5 -- $result)
                    builtin history merge
                    builtin history add -- $cmd
                    command ukrop hook-cmd --cmd "$cmd" --exit-code 0 --cwd "$PWD" &>/dev/null &
                    eval $cmd
                case 'ssh:*'
                    set -l ssh_args (string sub -s 5 -- $result)
                    command ukrop hook-ssh --host "$ssh_args" &>/dev/null &
                    ssh (string split ' ' -- $ssh_args)
            end
        end
    else
        command ukrop $argv
    end
end

# Ctrl+R binding for command history
function __ukrop_ctrl_r
    set -l result (command ukrop search </dev/tty)
    or return
    set -l edit 0
    if test -n "$result"
        if string match -q 'edit:*' -- $result
            set result (string sub -s 6 -- $result)
            set edit 1
        end
        switch $result
            case 'cd:*'
                commandline -r "cd -- "(string sub -s 4 -- $result)
            case 'run:*'
                commandline -r (string sub -s 5 -- $result)
            case 'ssh:*'
                commandline -r "ssh "(string sub -s 5 -- $result)
        end
        if test $edit -eq 0
            commandline -f execute
        else
            commandline -f repaint
        end
        return
    end
    commandline -f repaint
end
bind \cr __ukrop_ctrl_r

alias u=ukrop
"#
}
