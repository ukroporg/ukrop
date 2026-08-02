pub fn init_script() -> &'static str {
    r#"
# ukrop shell integration for bash
__ukrop_last_cmd=""
__ukrop_cmd_start=""
__ukrop_hook_err="${TMPDIR:-/tmp}/ukrop-hook-$$.err"
__ukrop_hook_warned=0
__ukrop_preexec() {
    __ukrop_last_cmd="$(HISTTIMEFORMAT= history 1 | sed 's/^[ ]*[0-9]*[ ]*//')"
    __ukrop_cmd_start=$SECONDS
}
trap '__ukrop_preexec' DEBUG

__ukrop_hook() {
    local exit_code=$?
    command ukrop hook --shell-id "$$" -- "$PWD" 2>/dev/null
    if [ -n "$__ukrop_last_cmd" ]; then
        local duration_args=""
        if [ -n "$__ukrop_cmd_start" ]; then
            local duration_ms=$(( (SECONDS - __ukrop_cmd_start) * 1000 ))
            duration_args="--duration-ms $duration_ms"
        fi
        { command ukrop hook-cmd --cmd "$__ukrop_last_cmd" --exit-code "$exit_code" --cwd "$PWD" $duration_args 2>>"$__ukrop_hook_err" & disown; } 2>/dev/null
        __ukrop_last_cmd=""
        __ukrop_cmd_start=""
    fi
    if [ "$__ukrop_hook_warned" = "0" ] && [ -s "$__ukrop_hook_err" ]; then
        __ukrop_hook_warned=1
        echo "ukrop: command tracking error (see $__ukrop_hook_err):" >&2
        head -5 "$__ukrop_hook_err" >&2
    fi
}

if [[ ! "$PROMPT_COMMAND" == *"__ukrop_hook"* ]]; then
    PROMPT_COMMAND="__ukrop_hook${PROMPT_COMMAND:+;$PROMPT_COMMAND}"
fi

ukrop() {
    if [ $# -eq 0 ] || [ "$1" = "cd" ] || [ "$1" = "run" ] || [ "$1" = "ssh" ]; then
        local result
        result="$(command ukrop "$@")" || return $?
        if [ -n "$result" ]; then
            # Shift+Enter: put text on command line for editing
            if [[ "$result" == edit:* ]]; then
                result="${result#edit:}"
                local cmd
                case "$result" in
                    cd:*)  cmd="cd -- '${result#cd:}'" ;;
                    run:*) cmd="${result#run:}" ;;
                    ssh:*) cmd="ssh ${result#ssh:}" ;;
                esac
                read -ei "$cmd" -p "" __ukrop_edit_cmd
                eval "$__ukrop_edit_cmd"
                return
            fi
            case "$result" in
                cd:*)
                    builtin cd -- "${result#cd:}" || return $?
                    ;;
                run:*)
                    local cmd="${result#run:}"
                    history -s "$cmd"
                    { command ukrop hook-cmd --cmd "$cmd" --exit-code 0 --cwd "$PWD" &>/dev/null & disown; } 2>/dev/null
                    eval "$cmd"
                    ;;
                ssh:*)
                    local ssh_args="${result#ssh:}"
                    { command ukrop hook-ssh --host "$ssh_args" &>/dev/null & disown; } 2>/dev/null
                    ssh $ssh_args
                    ;;
            esac
        fi
    else
        command ukrop "$@"
    fi
}

# Ctrl+R binding for command history
__ukrop_ctrl_r() {
    local result
    result="$(command ukrop run </dev/tty)" || return $?
    if [ -n "$result" ]; then
        # Strip edit: prefix — Ctrl+R always puts text on command line
        [[ "$result" == edit:* ]] && result="${result#edit:}"
        case "$result" in
            cd:*)
                READLINE_LINE="cd -- '${result#cd:}'"
                READLINE_POINT=${#READLINE_LINE}
                ;;
            run:*)
                READLINE_LINE="${result#run:}"
                READLINE_POINT=${#READLINE_LINE}
                ;;
            ssh:*)
                READLINE_LINE="ssh ${result#ssh:}"
                READLINE_POINT=${#READLINE_LINE}
                ;;
        esac
    fi
}
bind -x '"\C-r": __ukrop_ctrl_r'

alias u=ukrop
"#
}
