_sarges_unpacker() {
local commands="create extract list verify stats --help --version"
    local options="-r -m= --json --dry-run --overwrite --skip-existing --progress --quiet --verbose"
    COMPREPLY=( $(compgen -W "$commands $options" -- "${COMP_WORDS[COMP_CWORD]}") )
}
complete -F _sarges_unpacker sarges-unpacker
