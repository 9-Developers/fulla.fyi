#!/bin/sh
# Reject cargo-about's canonical MIT fallback, not Apache's example appendix.
set -eu

notices="${1:-licenses/rust.txt}"
if [ ! -r "$notices" ]; then
	echo "Cannot read license notices: $notices" >&2
	exit 1
fi

status=0
grep -niE 'Copyright.*(<year>|<copyright holders>)' "$notices" || status=$?
case "$status" in
0)
	echo "Placeholder copyright notice found. Clarify the upstream license in about.toml and regenerate." >&2
	exit 1
	;;
1) exit 0 ;;
*) exit "$status" ;;
esac
