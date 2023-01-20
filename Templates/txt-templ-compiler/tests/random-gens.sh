#!/usr/bin/env bash

# Usage: DISPLAY=true ./random-gens.sh 20000 grammar.bnf ../target/debug/test

set -eo pipefail
set -e

if ! [[ -x "$(command -v bnfgen)" ]]; then
  echo >&2 "Error: bnfgen is not installed. Visit https://baturin.org/tools/bnfgen/ to install it."
  exit 1
fi

x=1
while [ $x -le $1 ]
do
  bnfgen --separator "" $2 | $3
  x=$(( $x + 1 ))
done

