#!/usr/bin/env bash
# Test retained release images against an isolated copy of the migration sequence.
set -euo pipefail
old=${1:?usage: rehearse-rollback.sh OLD_IMAGE@sha256:... STAGED_IMAGE@sha256:...}
new=${2:?missing staged image}
for image in "$old" "$new"; do
	[[ $image =~ ^[^@[:space:]]+@sha256:[0-9a-f]{64}$ ]] || exit 2
done
net=$(docker network create "fulla-rehearsal-$$")
db=
web=
cleanup() {
	if [[ -n $web ]]; then docker rm -f "$web" >/dev/null; fi
	if [[ -n $db ]]; then docker rm -fv "$db" >/dev/null; fi
	docker network rm "$net" >/dev/null
}
trap cleanup EXIT
db=$(docker run -d --network "$net" --network-alias db -e POSTGRES_PASSWORD=rehearsal \
	--health-cmd='pg_isready -h 127.0.0.1 -U postgres' --health-interval=1s postgres:18)
for ((i = 0; i < 60; i++)); do
	[[ $(docker inspect -f '{{.State.Health.Status}}' "$db") != healthy ]] || break
	sleep 1
done
args=(--platform linux/amd64 --network "$net" -e DATABASE_URL=postgres://postgres:rehearsal@db/postgres)
migrate() { docker run --rm "${args[@]}" "$1" migrate; }
health() {
	web=$(docker run -d "${args[@]}" -p 127.0.0.1::3000 "$1")
	local address
	address=$(docker port "$web" 3000/tcp)
	curl --silent --show-error --fail --retry 12 --retry-connrefused --retry-delay 1 "http://$address/health"
	docker rm -f "$web" >/dev/null
	web=
}
migrate "$old"
health "$old"
migrate "$new"
health "$new"
migrate "$old"
health "$old"
echo "Previous image passed its pre-deploy migration and readiness check after the staged upgrade."
