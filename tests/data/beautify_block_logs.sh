#!/bin/sh

pwd=$(pwd)
cd ./block_logs

if [ -d ../beautified_logs ]; then
    echo "Beautified dir exists already! Welcome back :)";
else
    echo "Creating ../beautified_logs dir"
    mkdir ../beautified_logs;
fi

for item in *; do
    if [ -f "${item}" ]; then
        beautified=../beautified_logs/$item
        touch $beautified;
        js-beautify $item > $beautified;
        echo "Beautified ${item}";
    fi
done

cd $pwd
