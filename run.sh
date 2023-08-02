#!/bin/sh

original_stty_config=$(stty -g)

die() {
    echo $1
    stty "${orginal_stty_config}"    
    exit 1
}

# File doesn't exist, compile and copy
if [ -e "keyboard-bridge" ]; then :; else
    echo "Compiling"
    cargo install --path . || die "Failed to compile"
    echo "Finished"
fi

echo 'Running. Exit with <Enter>~.<Backspace><Backspace><Backspace> (a.k.a. ^M~.^H^H^H)'
stty raw -echo
./keyboard-bridge || die "Exited with non-0 exit code"
stty "${original_stty_config}"