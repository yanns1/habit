_habit_completions() {
    local cur opt
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"

    # Completion of commands.
    if [[ $COMP_CWORD == 1 ]]; then
        COMPREPLY=( $(compgen -W 'new edit delete list log suspend resume show help' -- "$cur") )
        return 0
    fi

    # Completion of command parameters.
    opt="${COMP_WORDS[1]}"
    case $opt in
    "edit")
        if [[ $COMP_CWORD == 2 ]]; then
            local habits=$(habit list)
            COMPREPLY=( $(compgen -W "${habits[@]}" -- "$cur") )
        fi

        if [[ $COMP_CWORD == 3 ]]; then
            COMPREPLY=( $(compgen -W 'name description days at' -- "$cur") )
        fi
        return 0
        ;;

    "delete")
        if [[ $COMP_CWORD == 2 ]]; then
            local habits=$(habit list)
            COMPREPLY=( $(compgen -W "${habits[@]}" -- "$cur") )
        fi
        return 0
        ;;

    "list")
        if [[ $COMP_CWORD == 2 ]]; then
            COMPREPLY=( $(compgen -W '-v' -- "$cur") )
        fi
        return 0
        ;;

    "log")
        if [[ $COMP_CWORD == 2 ]]; then
            local habits=$(habit list)
            COMPREPLY=( $(compgen -W "${habits[@]}" -- "$cur") )
        fi
        return 0
        ;;

    "suspend")
        if [[ $COMP_CWORD == 2 ]]; then
            local habits=$(habit list)
            COMPREPLY=( $(compgen -W "${habits[@]}" -- "$cur") )
        fi
        return 0
        ;;

    "resume")
        if [[ $COMP_CWORD == 2 ]]; then
            local habits=$(habit list)
            COMPREPLY=( $(compgen -W "${habits[@]}" -- "$cur") )
        fi
        return 0
        ;;

    "show")
        if [[ $COMP_CWORD == 2 ]]; then
            local habits=$(habit list)
            COMPREPLY=( $(compgen -W "${habits[@]} -h" -- "$cur") )
        fi
        return 0
        ;;
    esac
}

complete -F _habit_completions habit -o bashdefault -o default
