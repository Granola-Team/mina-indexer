#!/usr/bin/env nix-shell
#! nix-shell -i bash -p nodePackages.js-beautify

# provide source dir via first arg
source_dir=$1
beautified_dir="$(pwd)"/beautified_$1

# check source dir exists
if ! [ -d $source_dir ]; then
    echo "Directory ${source_dir} does not exist";
    exit 1
fi

# create beautified dir if needed
if [ -d $beautified_dir ]; then
    echo "Beautified dir exists already! Welcome back :)"
else
    echo "Creating ${beautified_dir} dir";
    mkdir $beautified_dir;
fi

cd $source_dir

# beautify blocks in the source dir
for item in *; do
    if [ -f "${item}" ]; then
        beautified=$beautified_dir/$item
        # touch $beautified;
        js-beautify $item > $beautified;
        echo "Beautified ${item}";
    fi
done

cd ..
