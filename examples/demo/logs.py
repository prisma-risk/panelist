#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Feeds the Loki stream that the `full_dashboard` example queries.

`full_dashboard.rs` has one panel backed by Loki:

    {service="geoip"} |= "resolution failed"

so this pushes a `service="geoip"` stream containing lines that match that
filter, alongside enough ordinary traffic that the filter is doing visible
work rather than matching everything.

On startup it backfills an hour of history. Prometheus can only show what
it has scraped since boot, but Loki accepts backdated pushes, so the log
panel is populated the moment the stack is up instead of after an hour of
waiting.

Only the standard library is used, so the container needs no build step.
"""

import json
import os
import random
import sys
import time
import urllib.error
import urllib.request

BACKFILL_SECONDS = 3600
PUSH_INTERVAL_SECONDS = 3.0
READY_TIMEOUT_SECONDS = 120

CITIES = (
    ("203.0.113.17", "Lisbon", "PT"),
    ("198.51.100.42", "Osaka", "JP"),
    ("192.0.2.88", "Nairobi", "KE"),
    ("203.0.113.201", "Santiago", "CL"),
    ("198.51.100.7", "Reykjavik", "IS"),
    ("192.0.2.140", "Toronto", "CA"),
)

FAILURES = (
    "resolution failed for {ip}: upstream database timeout after 250ms",
    "resolution failed for {ip}: no prefix match in the v4 table",
    "resolution failed for {ip}: upstream returned 503, falling back to cache",
    "resolution failed for {ip}: cache entry expired and refresh is throttled",
)

SUCCESSES = (
    "resolved {ip} to {city}, {country} in {ms}ms",
    "cache hit for {ip} ({city}, {country})",
    "refreshed prefix table, {ms} entries changed",
)


def line(rng: random.Random) -> tuple[str, str]:
    """One log line and its level. Roughly one in five is a failure."""
    ip, city, country = rng.choice(CITIES)
    if rng.random() < 0.2:
        template = rng.choice(FAILURES)
        level = "error"
    else:
        template = rng.choice(SUCCESSES)
        level = "info"
    return level, template.format(
        ip=ip, city=city, country=country, ms=rng.randint(2, 240)
    )


def push(url: str, entries: list[tuple[int, str, str]]) -> None:
    """Send entries as one stream per level.

    Loki requires the entries within a stream to be ordered by timestamp,
    and splitting by level is also what makes the `level` label usable as a
    filter in the panel.
    """
    by_level: dict[str, list[list[str]]] = {}
    for timestamp_ns, level, text in sorted(entries):
        by_level.setdefault(level, []).append([str(timestamp_ns), text])

    payload = {
        "streams": [
            {"stream": {"service": "geoip", "level": level}, "values": values}
            for level, values in by_level.items()
        ]
    }
    request = urllib.request.Request(
        url,
        data=json.dumps(payload).encode(),
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(request, timeout=10) as response:
        response.read()


def wait_for_loki(base_url: str) -> bool:
    deadline = time.time() + READY_TIMEOUT_SECONDS
    while time.time() < deadline:
        try:
            with urllib.request.urlopen(f"{base_url}/ready", timeout=5) as response:
                if response.status == 200:
                    return True
        except (urllib.error.URLError, OSError):
            pass
        time.sleep(2)
    return False


def main() -> int:
    base_url = os.environ.get("LOKI_URL", "http://loki:3100")
    push_url = f"{base_url}/loki/api/v1/push"
    rng = random.Random(1729)

    if not wait_for_loki(base_url):
        print(f"loki never became ready at {base_url}", file=sys.stderr)
        return 1

    now = time.time()
    backfill: list[tuple[int, str, str]] = []
    moment = now - BACKFILL_SECONDS
    while moment < now:
        moment += rng.uniform(0.5, 4.0)
        level, text = line(rng)
        backfill.append((int(moment * 1e9), level, text))
    push(push_url, backfill)
    print(f"backfilled {len(backfill)} lines", file=sys.stderr)

    while True:
        time.sleep(PUSH_INTERVAL_SECONDS)
        moment = time.time()
        entries = []
        for index in range(rng.randint(1, 5)):
            level, text = line(rng)
            entries.append((int((moment + index * 1e-3) * 1e9), level, text))
        try:
            push(push_url, entries)
        except (urllib.error.URLError, OSError) as error:
            print(f"push failed: {error}", file=sys.stderr)


if __name__ == "__main__":
    raise SystemExit(main())
