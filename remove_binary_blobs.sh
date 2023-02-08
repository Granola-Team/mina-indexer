#!/usr/bin/env nix-shell
#! nix-shell -i bash -p jq

LOGS_DIR=$1
OUT_DIR=$2
echo "scanning block logs in $LOGS_DIR for sok_digest or vrf_result"
for BLOCK_LOG in $LOGS_DIR/*.json;
do
    echo $BLOCK_LOG
    OUT_FILE=$OUT_DIR/$(basename $BLOCK_LOG)
    jq 'walk(
        if type == "object" and has("sok_digest") 
        then del(.sok_digest) 
        else 
            if type == "object" and has("vrf_result") 
            then del(.vrf_result) 
            else . 
            end 
        end
    )' $BLOCK_LOG > $OUT_FILE
done