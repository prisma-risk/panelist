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
# `trap cleanup EXIT INT TERM` is NOT enough: on INT/TERM bash runs the
# handler and then *resumes execution* where it left off, so a killed run
# tears down its own Grafana, keeps going, and exits 0 — a false PASS, which
# is worse than the leaked container this trap was added to prevent. The
# handler has to exit itself. `cleanup` then runs twice (handler, then EXIT
# trap); it is idempotent, so that is harmless.
trap cleanup EXIT
trap 'cleanup; exit 130' INT
trap 'cleanup; exit 143' TERM

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
# Everything else missing, value-altered (leaves), retyped (containers), or
# unreachable because an ancestor container came back as a scalar ("blocked")
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

# Resolves $path in $doc one segment at a time, without ever hard-erroring.
#
# Deliberately not `getpath`: getpath is null-safe through a *missing* or
# *null* intermediate, but not through a type mismatch. Indexing through a
# non-null scalar - `{"a":"hello"} | getpath(["a","b"])` - raises
# `Cannot index string with string`, and jq exits non-zero. Under
# `set -euo pipefail` that aborts the whole script: this golden never
# reports its result, and every golden later in the loop is silently
# skipped with nothing in the output saying so. That is exactly the shape
# of drift this script exists to catch - a future Grafana restructuring a
# nested config sub-object - so it must be reported, not fatal.
#
# Emits one of:
#   {state: "present", value: <value at $path>}
#   {state: "missing"}
#   {state: "blocked", at: <path of the scalar ancestor>, actual: <its type>}
def resolve($doc; $path):
  reduce range(0; $path | length) as $depth (
    {state: "present", value: $doc, at: []};
    if .state != "present" then
      .
    else
      ($path[$depth]) as $seg
      | .value as $here
      | (.at + [$seg]) as $next_at
      | if $here == null then
          {state: "missing"}
        elif ($here | type) == "object" then
          (if ($seg | type) == "string" and ($here | has($seg))
           then {state: "present", value: $here[$seg], at: $next_at}
           else {state: "missing"}
           end)
        elif ($here | type) == "array" then
          (if ($seg | type) == "number" and $seg >= 0 and $seg < ($here | length)
           then {state: "present", value: $here[$seg], at: $next_at}
           else {state: "missing"}
           end)
        else
          {state: "blocked", at: .at, actual: ($here | type)}
        end
    end
  );

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
        | resolve($got; $path) as $found
        | if $found.state == "blocked" then
            {path: ($path | path_str), kind: "blocked", expected: $expected,
             actual: "\($found.actual) at \($found.at | path_str)"}
          elif $found.state == "missing" then
            {path: ($path | path_str), kind: "dropped", expected: $expected, actual: null}
          elif $found.value != $expected then
            {path: ($path | path_str), kind: "altered", expected: $expected, actual: $found.value}
          else
            empty
          end
      ] + [
        $container_paths[] as $path
        | ($sent | getpath($path) | type) as $expected_type
        | resolve($got; $path) as $found
        | if $found.state == "blocked" then
            {path: ($path | path_str), kind: "blocked", expected: $expected_type,
             actual: "\($found.actual) at \($found.at | path_str)"}
          elif $found.state == "missing" then
            {path: ($path | path_str), kind: "dropped", expected: $expected_type, actual: null}
          elif ($found.value | type) != $expected_type then
            {path: ($path | path_str), kind: "retyped", expected: $expected_type, actual: ($found.value | type)}
          else
            empty
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
    echo "FAIL: ${name} — Grafana dropped, altered, retyped, or blocked access to ${problem_count} of $((leaves_checked + containers_checked)) properties (${leaves_checked} leaves, ${containers_checked} empty containers):"
    echo "${result}" | jq -r '.problems[] | "  [\(.kind)] \(.path)\n    expected: \(.expected | tojson)\n    actual:   \(.actual | tojson)"'
    overall_status=1
  fi
done

echo
if [ "${overall_status}" -eq 0 ]; then
  echo "==> All goldens round-tripped through Grafana with every property preserved."
else
  echo "==> Grafana dropped, altered, retyped, or blocked access to properties Panelist emitted. See above." >&2
fi

exit "${overall_status}"
