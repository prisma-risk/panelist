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
trap cleanup EXIT INT TERM

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

# Walks $sent in two disjoint categories and checks each against $got to a
# standard appropriate for what Grafana is allowed to do to it:
#
#   - Leaves (scalars, including null and false): checked for presence AND
#     exact value. A leaf that is missing, or present with a different
#     value, is a genuine drop/alteration.
#   - Empty containers ({} or [] in $sent): checked for presence and
#     container *type* only, not value. Grafana routinely adds entries into
#     an empty container on save — annotations.list: [] is a documented
#     case, where Grafana injects its built-in "Annotations & Alerts" entry
#     — and that is exactly the kind of addition this script's stated
#     contract already ignores everywhere else (the walk only ever visits
#     paths $sent has). Checking value equality here would make every one
#     of those legitimate additions read as a false "altered". So an empty
#     container only fails if the key is gone outright, or if what is there
#     is no longer the same container type (e.g. an object where Panelist
#     sent an array).
#
# Grafana legitimately adds fields on save (ignored automatically for both
# categories: the walk only ever visits $sent's paths) and rewrites exactly
# four: the top-level id, version, and uid, and each panel's pluginVersion.
# Everything else missing, value-altered (leaves), or retyped (containers)
# is a genuine problem and gets reported.
#
# pluginVersion is excluded for completeness with the four documented
# rewrites, not because it has been observed. Panelist has never emitted a
# "pluginVersion" key in a golden dashboard, so this branch of is_excluded
# has not actually been exercised by either golden — it is here so a
# dashboard that later does emit one is not misreported as dropped.
cat >"${COMPARE_JQ}" <<'JQ'
def path_str:
  reduce .[] as $seg (""; . + (if ($seg | type) == "number" then "[\($seg)]" else ".\($seg)" end));

# A leaf is any node that is not itself a container. Deliberately not
# `paths(scalars)`: jq's paths(f) only emits a path when f's result at that
# path is truthy, and `scalars` echoes the value itself — so a `null` or
# `false` leaf produces a falsy result and paths(scalars) silently omits it.
# That would make a dropped-and-replaced-with-null value, or a dropped
# `false` flag, invisible to this check. Testing on *type* instead of
# truthiness catches every leaf regardless of its value.
def is_leaf:
  (type != "object") and (type != "array");

# An empty container is an object or array with no children of its own —
# `is_leaf` is false for these (they are containers), and `paths` never
# descends into them (there is nothing inside to descend into), so without
# a category of their own they would never be visited at all: invisible to
# the check in exactly the way null/false leaves were before this script
# started testing leaves by type instead of truthiness.
def is_empty_container:
  (type == "object" or type == "array") and (length == 0);

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
| [$sent | paths(is_leaf) | select(is_excluded | not)] as $leaf_paths
| [$sent | paths(is_empty_container) | select(is_excluded | not)] as $container_paths
| {
    leaves_checked: ($leaf_paths | length),
    containers_checked: ($container_paths | length),
    problems: (
      [
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
      ] + [
        $container_paths[] as $path
        | ($sent | getpath($path) | type) as $expected_type
        | if (exists_at($got; $path) | not) then
            {path: ($path | path_str), kind: "dropped", expected: $expected_type, actual: null}
          else
            ($got | getpath($path) | type) as $actual_type
            | if $actual_type != $expected_type then
                {path: ($path | path_str), kind: "retyped", expected: $expected_type, actual: $actual_type}
              else
                empty
              end
          end
      ]
    )
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

  leaves_checked="$(echo "${result}" | jq -r '.leaves_checked')"
  containers_checked="$(echo "${result}" | jq -r '.containers_checked')"
  problem_count="$(echo "${result}" | jq -r '.problems | length')"

  if [ "$((leaves_checked + containers_checked))" -eq 0 ]; then
    echo "FAIL: ${name} — 0 leaf properties and 0 empty containers found to check; the golden or the response is empty or degenerate" >&2
    overall_status=1
    continue
  fi

  if [ "${problem_count}" -eq 0 ]; then
    echo "PASS: ${name} — ${leaves_checked} leaf properties and ${containers_checked} empty containers checked, all preserved"
  else
    echo "FAIL: ${name} — Grafana dropped, altered, or retyped ${problem_count} of $((leaves_checked + containers_checked)) properties (${leaves_checked} leaves, ${containers_checked} empty containers):"
    echo "${result}" | jq -r '.problems[] | "  [\(.kind)] \(.path)\n    expected: \(.expected | tojson)\n    actual:   \(.actual | tojson)"'
    overall_status=1
  fi
done

echo
if [ "${overall_status}" -eq 0 ]; then
  echo "==> All goldens round-tripped through Grafana with every property preserved."
else
  echo "==> Grafana dropped, altered, or retyped properties Panelist emitted. See above." >&2
fi

exit "${overall_status}"
