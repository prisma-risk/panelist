#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# Round-trips every committed golden dashboard through a real Grafana
# instance and confirms Grafana kept every property Panelist emitted.
#
# Grafana returns HTTP 200 for a dashboard POST even when it silently drops
# a key it does not understand - it just discards it. A JSON diff of "what
# we meant to send" only proves intent, not acceptance. This script proves
# acceptance: it walks every leaf of the dashboard we sent and asserts that
# leaf still exists, unchanged, in the dashboard Grafana hands back.
#
# Usage: scripts/verify-grafana.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

GRAFANA_IMAGE="grafana/grafana-oss:13.0.2"
CONTAINER_NAME="panelist-verify-grafana"
GRAFANA_PORT=3000
BASE_URL="http://localhost:${GRAFANA_PORT}"
HEALTH_TIMEOUT_SECONDS=90
HEALTH_POLL_INTERVAL_SECONDS=2

GOLDEN_FILES=(
  "${REPO_ROOT}/crates/panelist/tests/golden/basic.json"
  "${REPO_ROOT}/crates/panelist/tests/golden/route_performance.json"
)

WORK_DIR="$(mktemp -d)"
COMPARE_JQ="${WORK_DIR}/compare.jq"

cleanup() {
  docker rm -f "${CONTAINER_NAME}" >/dev/null 2>&1 || true
  rm -rf "${WORK_DIR}"
}
trap cleanup EXIT

echo "==> Booting ${GRAFANA_IMAGE} on port ${GRAFANA_PORT}"
docker rm -f "${CONTAINER_NAME}" >/dev/null 2>&1 || true
docker run -d \
  --name "${CONTAINER_NAME}" \
  -p "${GRAFANA_PORT}:3000" \
  -e GF_AUTH_ANONYMOUS_ENABLED=true \
  -e GF_AUTH_ANONYMOUS_ORG_ROLE=Admin \
  "${GRAFANA_IMAGE}" >/dev/null

echo "==> Waiting for Grafana to report a healthy database (timeout ${HEALTH_TIMEOUT_SECONDS}s)"
elapsed=0
until curl -fsS "${BASE_URL}/api/health" 2>/dev/null | jq -e '.database == "ok"' >/dev/null 2>&1; do
  if [ "${elapsed}" -ge "${HEALTH_TIMEOUT_SECONDS}" ]; then
    echo "FAILED: Grafana did not report a healthy database within ${HEALTH_TIMEOUT_SECONDS}s." >&2
    echo "Container logs:" >&2
    docker logs "${CONTAINER_NAME}" >&2 || true
    exit 1
  fi
  sleep "${HEALTH_POLL_INTERVAL_SECONDS}"
  elapsed=$((elapsed + HEALTH_POLL_INTERVAL_SECONDS))
done
echo "==> Grafana is healthy"

# Walks every leaf (scalar or null) of $sent and asserts it is still present,
# with the same value, at the same path in $got. Grafana legitimately adds
# fields on save (ignored automatically: we only ever walk $sent's paths) and
# rewrites exactly four: the top-level id, version, and uid, and each panel's
# pluginVersion. Everything else that is missing or changed is a genuine drop
# or alteration and gets reported.
cat >"${COMPARE_JQ}" <<'JQ'
def path_str:
  reduce .[] as $seg (""; . + (if ($seg | type) == "number" then "[\($seg)]" else ".\($seg)" end));

def is_excluded:
  . as $path
  | ($path == ["id"])
    or ($path == ["version"])
    or ($path == ["uid"])
    or (($path | length) == 3 and $path[0] == "panels" and $path[2] == "pluginVersion");

def exists_at($doc; $path):
  if ($path | length) == 0 then
    true
  else
    ($doc | getpath($path[0:-1])) as $parent
    | ($path[-1]) as $key
    | if $parent == null then
        false
      elif ($parent | type) == "object" then
        $parent | has($key)
      elif ($parent | type) == "array" then
        ($key | type) == "number" and $key >= 0 and $key < ($parent | length)
      else
        false
      end
  end;

($sent[0]) as $sent
| ($got[0]) as $got
| [$sent | paths(scalars) | select(is_excluded | not)] as $leaf_paths
| {
    checked: ($leaf_paths | length),
    problems: [
      $leaf_paths[] as $path
      | ($sent | getpath($path)) as $expected
      | if (exists_at($got; $path) | not) then
          {path: ($path | path_str), kind: "dropped", expected: $expected, actual: null}
        else
          ($got | getpath($path)) as $actual
          | if $actual != $expected then
              {path: ($path | path_str), kind: "altered", expected: $expected, actual: $actual}
            else
              empty
            end
        end
    ]
  }
JQ

overall_status=0

for golden in "${GOLDEN_FILES[@]}"; do
  name="$(basename "${golden}")"
  uid="$(jq -r '.uid' "${golden}")"
  echo
  echo "==> Round-tripping ${name} (uid: ${uid})"

  request_body="${WORK_DIR}/${name}.request.json"
  jq -n --slurpfile dashboard "${golden}" '{dashboard: $dashboard[0], overwrite: true}' \
    >"${request_body}"

  post_response="${WORK_DIR}/${name}.post-response.json"
  post_status="$(curl -sS -o "${post_response}" -w '%{http_code}' \
    -X POST "${BASE_URL}/api/dashboards/db" \
    -H "Content-Type: application/json" \
    -d @"${request_body}")"

  if [ "${post_status}" != "200" ]; then
    echo "FAIL: ${name} — POST /api/dashboards/db returned HTTP ${post_status}" >&2
    cat "${post_response}" >&2
    overall_status=1
    continue
  fi

  returned_uid="$(jq -r '.uid' "${post_response}")"

  get_response="${WORK_DIR}/${name}.get-response.json"
  get_status="$(curl -sS -o "${get_response}" -w '%{http_code}' \
    "${BASE_URL}/api/dashboards/uid/${returned_uid}")"

  if [ "${get_status}" != "200" ]; then
    echo "FAIL: ${name} — GET /api/dashboards/uid/${returned_uid} returned HTTP ${get_status}" >&2
    cat "${get_response}" >&2
    overall_status=1
    continue
  fi

  got_dashboard="${WORK_DIR}/${name}.got-dashboard.json"
  jq '.dashboard' "${get_response}" >"${got_dashboard}"

  result="$(jq -n \
    --slurpfile sent "${golden}" \
    --slurpfile got "${got_dashboard}" \
    -f "${COMPARE_JQ}")"

  checked="$(echo "${result}" | jq -r '.checked')"
  problem_count="$(echo "${result}" | jq -r '.problems | length')"

  if [ "${problem_count}" -eq 0 ]; then
    echo "PASS: ${name} — ${checked} properties checked, all preserved"
  else
    echo "FAIL: ${name} — Grafana dropped or altered ${problem_count} of ${checked} properties:"
    echo "${result}" | jq -r '.problems[] | "  [\(.kind)] \(.path)\n    expected: \(.expected | tojson)\n    actual:   \(.actual | tojson)"'
    overall_status=1
  fi
done

echo
if [ "${overall_status}" -eq 0 ]; then
  echo "==> All goldens round-tripped through Grafana with every property preserved."
else
  echo "==> Grafana dropped or altered properties Panelist emitted. See above." >&2
fi

exit "${overall_status}"
