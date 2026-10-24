#!/bin/sh
# Gera o RPM do Redoma em dist/. Requer: cargo, gtk4-devel, libadwaita-devel
# e rpm-build (sudo dnf install cargo gtk4-devel libadwaita-devel rpm-build).
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
version=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)
topdir="$root/dist/rpmbuild"

cargo build --release --locked
rm -rf "$topdir"
mkdir -p "$topdir/SOURCES"
cp target/release/redoma \
   packaging/common/redoma.desktop \
   packaging/common/io.github.rodolfoaltenrath.Redoma.svg \
   LICENSE README.md SECURITY.md \
   "$topdir/SOURCES/"

rpmbuild -bb \
  --define "_topdir $topdir" \
  --define "redoma_version $version" \
  packaging/rpm/redoma.spec

find "$topdir/RPMS" -name '*.rpm' -exec cp {} dist/ \;
ls -1 dist/*.rpm
