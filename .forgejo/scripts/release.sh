#!/usr/bin/env bash
set -euo pipefail
umask 077
: "${REGISTRY_USER:?}" "${REGISTRY_TOKEN:?}" "${RAILWAY_PROJECT_ID:?}" "${RAILWAY_TOKEN:?}"
case "$RAILWAY_ENVIRONMENT" in
staging) run=${RELEASE_RUN_ID:?} ;;
production) run=$(node .forgejo/scripts/deploy.mjs verify-staging) ;;
*) exit 1 ;;
esac

sha=$(git rev-parse HEAD)
registry=${FORGEJO_API_URL#https://}
registry=${registry%%/*}
repo="$registry/${FORGEJO_REPOSITORY,,}"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
export DOCKER_CONFIG="$work/docker"
mkdir "$DOCKER_CONFIG"
printf '%s' "$REGISTRY_TOKEN" | crane auth login "$registry" -u "$REGISTRY_USER" --password-stdin
digest=$(node .forgejo/scripts/receipt.mjs read "$run" "$sha")
publish=false
if [[ -z $digest ]]; then
	if [[ $RAILWAY_ENVIRONMENT != staging ]]; then
		echo "No image receipt for successful staging run $run" >&2
		exit 1
	fi
	rustc -vV | grep -Fx 'host: x86_64-unknown-linux-gnu'
	NO_COLOR=1 tailwindcss --help | grep -Fx '≈ tailwindcss v4.3.5'
	export CARGO_TARGET_DIR="$work/target"
	cargo build --locked --release
	tag="$repo:staging-$sha-$run"
	bash .forgejo/scripts/package-image.sh "$CARGO_TARGET_DIR/release/fulla_fyi" "$tag"
	digest=$(crane digest "$tag")
	publish=true
fi
[[ $digest =~ ^sha256:[0-9a-f]{64}$ ]]
[[ $(crane digest "$repo@$digest") == "$digest" ]]
railway config plan --environment "$RAILWAY_ENVIRONMENT" --out "$work/plan.json"
railway config apply --environment "$RAILWAY_ENVIRONMENT" --plan "$work/plan.json" --yes
RELEASE_IMAGE="$repo@$digest" node .forgejo/scripts/deploy.mjs deploy
if [[ $publish == true ]]; then
	node .forgejo/scripts/receipt.mjs publish "$run" "$sha" "$digest"
fi
