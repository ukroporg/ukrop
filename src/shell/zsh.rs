pub fn init_script() -> &'static str {
    r#"
# ukrop shell integration for zsh
__ukrop_hook_err="${TMPDIR:-/tmp}/ukrop-hook-$$.err"
__ukrop_hook_warned=0

__ukrop_hook() {
    command ukrop hook -- "$PWD" 2>/dev/null
}

__ukrop_preexec() {
    __ukrop_last_cmd="$1"
    __ukrop_cmd_start=$SECONDS
}

__ukrop_precmd() {
    local exit_code=$?
    __ukrop_hook
    if [[ -n "$__ukrop_last_cmd" ]]; then
        local duration_ms
        if [[ -n "$__ukrop_cmd_start" ]]; then
            duration_ms=$(( ($SECONDS - __ukrop_cmd_start) * 1000 ))
            duration_ms=${duration_ms%.*}
        fi
        command ukrop hook-cmd --cmd "$__ukrop_last_cmd" --exit-code "$exit_code" --cwd "$PWD" ${duration_ms:+--duration-ms} ${duration_ms:+"$duration_ms"} 2>>"$__ukrop_hook_err" &!
        unset __ukrop_last_cmd __ukrop_cmd_start
    fi
    if (( __ukrop_hook_warned == 0 )) && [[ -s "$__ukrop_hook_err" ]]; then
        __ukrop_hook_warned=1
        echo "ukrop: command tracking error (see $__ukrop_hook_err):" >&2
        head -5 "$__ukrop_hook_err" >&2
    fi
}

if (( ${precmd_functions[(Ie)__ukrop_precmd]} == 0 )); then
    precmd_functions+=(__ukrop_precmd)
fi

if (( ${preexec_functions[(Ie)__ukrop_preexec]} == 0 )); then
    preexec_functions+=(__ukrop_preexec)
fi

ukrop() {
    if [[ $# -eq 0 ]] || [[ "$1" == "cd" ]] || [[ "$1" == "run" ]] || [[ "$1" == "ssh" ]]; then
        local result
        result="$(command ukrop "$@")" || return $?
        if [[ -n "$result" ]]; then
            # Shift+Enter: put text on command line for editing
            if [[ "$result" == edit:* ]]; then
                result="${result#edit:}"
                local cmd
                case "$result" in
                    cd:*)  cmd="cd -- ${(q)result#cd:}" ;;
                    run:*) cmd="${result#run:}" ;;
                    ssh:*) cmd="ssh ${result#ssh:}" ;;
                esac
                print -z -- "$cmd"
                return
            fi
            case "$result" in
                cd:*)
                    builtin cd -- "${result#cd:}" || return $?
                    ;;
                run:*)
                    local cmd="${result#run:}"
                    print -s -- "$cmd"
                    command ukrop hook-cmd --cmd "$cmd" --exit-code 0 --cwd "$PWD" &>/dev/null &!
                    eval "$cmd"
                    ;;
                ssh:*)
                    local ssh_args="${result#ssh:}"
                    command ukrop hook-ssh --host "$ssh_args" &>/dev/null &!
                    ssh ${=ssh_args}
                    ;;
            esac
        fi
    else
        command ukrop "$@"
    fi
}

# Ctrl+R binding for command history
__ukrop_ctrl_r() {
    local result edit=0
    result="$(command ukrop run </dev/tty)" || return $?
    if [[ -n "$result" ]]; then
        if [[ "$result" == edit:* ]]; then
            result="${result#edit:}"
            edit=1
        fi
        case "$result" in
            cd:*)
                LBUFFER="cd -- ${(q)result#cd:}"
                RBUFFER=""
                ;;
            run:*)
                LBUFFER="${result#run:}"
                RBUFFER=""
                ;;
            ssh:*)
                LBUFFER="ssh ${result#ssh:}"
                RBUFFER=""
                ;;
        esac
        zle reset-prompt
        if (( edit == 0 )); then
            zle accept-line
        fi
        return
    fi
    zle reset-prompt
}
zle -N __ukrop_ctrl_r
bindkey '^R' __ukrop_ctrl_r

alias u=ukrop
"#
}
