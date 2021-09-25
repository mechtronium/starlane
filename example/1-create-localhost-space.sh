#!/bin/bash

export STARLANE_DATA=$HOME/tmp/starlane-cli/data
export STARLANE_CACHE=$CACHE/tmp/starlane-cli/cache

starlane create "localhost<Space>"

echo 
echo 
echo "go to http://localhost:8080/"
echo "You should see that STARLANE has found the localhost space, but it is not yet configured"

