#!/usr/bin/env sh
set -eu

docs_root=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
destination=${1:-"$docs_root/book"}
mdbook_bin=${MDBOOK:-mdbook}

# Every chapter must exist in both languages before the site can be built.
for chapter in "$docs_root"/src/*.md; do
    name=${chapter##*/}
    test -f "$docs_root/zh-CN/src/$name" || {
        echo "Missing Chinese chapter: $name" >&2
        exit 1
    }
done
for chapter in "$docs_root"/zh-CN/src/*.md; do
    name=${chapter##*/}
    test -f "$docs_root/src/$name" || {
        echo "Missing English chapter: $name" >&2
        exit 1
    }
done

mkdir -p "$destination"
destination=$(CDPATH= cd -- "$destination" && pwd)
"$mdbook_bin" build "$docs_root" -d "$destination/en"
"$mdbook_bin" build "$docs_root/zh-CN" -d "$destination/zh-CN"
cp "$docs_root/index.html" "$destination/index.html"
