#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Synthetic Prometheus exporter for the Panelist example dashboards.

Every metric name, label, and job here exists because one of the six
examples in ``crates/panelist/examples`` queries it. The point of the demo
stack is that the dashboards a reader builds resolve against real data, so
this file's job is to make each of those queries return something plausible
rather than an empty result.

Two properties matter more than realism:

*Counters must never decrease.* Prometheus treats any decrease as a counter
reset and ``rate()`` silently reports a spike. So values are accumulated
forward in fixed steps and never recomputed from a formula that could dip.

*Histogram buckets must stay cumulative.* ``histogram_quantile`` reads
``_bucket`` as a cumulative distribution: the count for a given ``le`` must
include every observation below it. Each step therefore adds the same batch
of requests to every bucket at or above the sampled latency, which keeps
the series non-decreasing both over time and across ``le``.

Only the standard library is used, so the container needs no build step.
"""

import math
import os
import sys
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

# Grafana's heatmap panel and `histogram_quantile` both read these
# boundaries straight off the `le` label, so they are the conventional
# Prometheus client defaults rather than anything tuned for this demo.
BUCKETS = (0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0)

# The accumulation grid. One second keeps the curves smooth at any scrape
# interval; the work per step is a few dozen float operations, so a stack
# left running for a day costs well under a second of catch-up.
STEP_SECONDS = 1.0

# A process that has been suspended (a laptop lid, a paused container)
# could otherwise try to replay hours of steps inside a single scrape.
MAX_STEPS_PER_ADVANCE = 100_000


def wave(t: float, seed: float) -> float:
    """A positive, slowly varying multiplier around 1.0.

    Three sine terms at different periods keep the shape from looking
    periodic over a screenshot-length window. The amplitudes sum to 0.60,
    so the result stays in [0.40, 1.60] and never reaches zero: an
    instantaneous rate that could go negative would produce a decreasing
    counter, which is the one thing this model must not do.
    """
    return (
        1.0
        + 0.35 * math.sin(t / 180.0 + seed)
        + 0.18 * math.sin(t / 47.0 + seed * 2.1)
        + 0.07 * math.sin(t / 13.0 + seed * 3.7)
    )


def lognormal_cdf(value: float, median: float, sigma: float) -> float:
    """Share of requests faster than ``value`` for a log-normal latency."""
    if value <= 0.0:
        return 0.0
    return 0.5 * (1.0 + math.erf(math.log(value / median) / (sigma * math.sqrt(2.0))))


class Counter:
    """A monotonic counter accumulated forward one step at a time."""

    def __init__(self, name: str, labels: dict[str, str], rate: float, seed: float):
        self.name = name
        self.labels = labels
        self.rate = rate
        self.seed = seed
        self.value = 0.0

    def step(self, t: float) -> float:
        added = self.rate * wave(t, self.seed) * STEP_SECONDS
        self.value += added
        return added


class Histogram:
    """A cumulative histogram whose median drifts slowly over time.

    A static distribution would render every latency panel as a flat line.
    Letting the median wander makes p50/p95/p99 move independently, which
    is what the latency panels in the examples are built to show. The
    drift only ever changes how *new* observations are bucketed; counts
    already recorded are never revised, so the series stays cumulative.
    """

    def __init__(
        self,
        name: str,
        labels: dict[str, str],
        rate: float,
        median: float,
        sigma: float,
        seed: float,
    ):
        self.name = name
        self.labels = labels
        self.rate = rate
        self.median = median
        self.sigma = sigma
        self.seed = seed
        self.buckets = [0.0] * len(BUCKETS)
        self.count = 0.0
        self.total = 0.0

    def step(self, t: float) -> None:
        observations = self.rate * wave(t, self.seed) * STEP_SECONDS
        if observations <= 0.0:
            return
        # ±35% of the base median, so a p95 panel has visible shape without
        # the series ever looking like it changed units.
        median = self.median * (1.0 + 0.35 * math.sin(t / 210.0 + self.seed * 1.7))
        for index, boundary in enumerate(BUCKETS):
            self.buckets[index] += observations * lognormal_cdf(
                boundary, median, self.sigma
            )
        self.count += observations
        self.total += observations * median * math.exp(self.sigma**2 / 2.0)


# Request mix per route. Every share is a fraction of the route's own rate,
# and the status codes are chosen so `route_performance`'s `status=~"2.."`,
# `"4.."`, and `"5.."` filters each match something on most routes.
ROUTES = (
    ("/api/v1/lookup", 18.0, 0.085, 0.55, {"200": 0.958, "404": 0.035, "500": 0.007}),
    ("/api/v1/batch", 4.2, 0.620, 0.70, {"200": 0.930, "429": 0.048, "503": 0.022}),
    ("/api/v1/report", 2.4, 1.350, 0.80, {"200": 0.880, "404": 0.060, "500": 0.060}),
    ("/healthz", 30.0, 0.0035, 0.35, {"200": 1.000}),
    ("/login", 6.5, 0.210, 0.60, {"200": 0.900, "401": 0.090, "500": 0.010}),
)

GEOIP_STATUS = {"200": 0.965, "400": 0.022, "503": 0.013}
GEOIP_OUTCOME = {"hit": 0.820, "miss": 0.150, "error": 0.030}


def seed_for(*parts: object) -> float:
    """A stable seed, so two scrape targets differ but a restart does not.

    ``hash()`` on a str is salted per process, which would reshuffle every
    series on restart and make two runs of the render script produce
    unrelated-looking dashboards.
    """
    import zlib

    key = "|".join(str(part) for part in parts).encode()
    return (zlib.crc32(key) % 100_000) / 100_000.0 * math.tau


def build_app(port: int) -> tuple[list[Counter], list[Histogram]]:
    """Metrics for `basic`, `prometheus`, and `route_performance`."""
    counters: list[Counter] = [
        Counter("requests_total", {}, 42.0, seed_for(port, "requests")),
        Counter("errors_total", {}, 1.35, seed_for(port, "errors")),
    ]
    histograms: list[Histogram] = [
        Histogram(
            "request_duration_seconds",
            {},
            42.0,
            0.120,
            0.60,
            seed_for(port, "duration"),
        )
    ]
    for route, rate, median, sigma, statuses in ROUTES:
        for status, share in statuses.items():
            counters.append(
                Counter(
                    "http_requests_total",
                    {"route": route, "status": status},
                    rate * share,
                    seed_for(port, route, status),
                )
            )
        histograms.append(
            Histogram(
                "http_request_duration_seconds",
                {"route": route},
                rate,
                median,
                sigma,
                seed_for(port, route, "latency"),
            )
        )
    return counters, histograms


def build_geoip(port: int) -> tuple[list[Counter], list[Histogram]]:
    """Metrics for `full_dashboard`."""
    counters = [
        Counter(
            "geoip_http_requests_total",
            {"status": status},
            24.0 * share,
            seed_for(port, "geoip", status),
        )
        for status, share in GEOIP_STATUS.items()
    ]
    counters += [
        Counter(
            "geoip_resolutions_total",
            {"outcome": outcome},
            21.0 * share,
            seed_for(port, "resolution", outcome),
        )
        for outcome, share in GEOIP_OUTCOME.items()
    ]
    histograms = [
        Histogram(
            "geoip_http_request_duration_seconds",
            {},
            24.0,
            0.070,
            0.50,
            seed_for(port, "geoip", "latency"),
        )
    ]
    return counters, histograms


def build_checkout(port: int) -> tuple[list[Counter], list[Histogram]]:
    """Metrics for `layout`."""
    counters = [
        Counter("checkout_requests_total", {}, 9.5, seed_for(port, "checkout")),
        Counter("checkout_errors_total", {}, 0.22, seed_for(port, "checkout-errors")),
    ]
    return counters, []


BUILDERS = {"app": build_app, "geoip": build_geoip, "checkout": build_checkout}


class Instance:
    """One scrape target: a port, a role, and that role's series."""

    def __init__(self, port: int, role: str, started_at: float):
        self.port = port
        self.role = role
        self.started_at = started_at
        self.steps = 0
        self.lock = threading.Lock()
        self.counters, self.histograms = BUILDERS[role](port)

    def advance(self, now: float) -> None:
        """Accumulate every step between the last scrape and ``now``.

        Advancing lazily on scrape rather than from a background thread
        makes the value a function of elapsed time alone, so a slow or
        missed scrape changes when a sample is taken but never what it
        would have been.
        """
        target = int((now - self.started_at) // STEP_SECONDS)
        with self.lock:
            budget = MAX_STEPS_PER_ADVANCE
            while self.steps < target and budget > 0:
                self.step_all(self.steps * STEP_SECONDS)
                self.steps += 1
                budget -= 1
            if self.steps < target:
                # Suspended for longer than the replay budget. Skipping the
                # backlog leaves a flat spot in the graph; replaying it would
                # instead stall the scrape and time it out. A flat spot is
                # the better failure, and the counters stay monotonic.
                self.steps = target

    def step_all(self, t: float) -> None:
        for counter in self.counters:
            counter.step(t)
        for histogram in self.histograms:
            histogram.step(t)

    def render(self) -> str:
        self.advance(time.time())
        lines: list[str] = []
        with self.lock:
            for name, counters in group_by_name(self.counters):
                lines += describe(name, "counter")
                for counter in counters:
                    labels = format_labels(counter.labels)
                    lines.append(f"{name}{labels} {counter.value:.6f}")
            for name, histograms in group_by_name(self.histograms):
                lines += describe(name, "histogram")
                for histogram in histograms:
                    for boundary, value in zip(BUCKETS, histogram.buckets):
                        bucket = dict(histogram.labels, le=format_boundary(boundary))
                        lines.append(
                            f"{name}_bucket{format_labels(bucket)} {value:.6f}"
                        )
                    overflow = dict(histogram.labels, le="+Inf")
                    lines.append(
                        f"{name}_bucket{format_labels(overflow)}"
                        f" {histogram.count:.6f}"
                    )
                    labels = format_labels(histogram.labels)
                    lines.append(f"{name}_sum{labels} {histogram.total:.6f}")
                    lines.append(f"{name}_count{labels} {histogram.count:.6f}")
        return "\n".join(lines) + "\n"


HELP = {
    "requests_total": "Requests handled by the demo service.",
    "errors_total": "Requests that failed.",
    "request_duration_seconds": "Request duration in seconds.",
    "http_requests_total": "Requests handled, by route and response status.",
    "http_request_duration_seconds": "Request duration in seconds, by route.",
    "geoip_http_requests_total": "GeoIP requests handled, by response status.",
    "geoip_http_request_duration_seconds": "GeoIP request duration in seconds.",
    "geoip_resolutions_total": "GeoIP resolution attempts, by outcome.",
    "checkout_requests_total": "Checkout requests handled.",
    "checkout_errors_total": "Checkout requests that failed.",
}


def describe(name: str, metric_type: str) -> list[str]:
    """The HELP and TYPE lines for one metric family.

    Prometheus rejects a scrape outright when a family declares TYPE twice,
    so these are emitted per family and never per series - which is what
    `group_by_name` exists to guarantee.
    """
    lines = []
    if name in HELP:
        lines.append(f"# HELP {name} {HELP[name]}")
    lines.append(f"# TYPE {name} {metric_type}")
    return lines


def group_by_name(series: list) -> list[tuple[str, list]]:
    """Group series by family name, preserving declaration order."""
    grouped: dict[str, list] = {}
    for item in series:
        grouped.setdefault(item.name, []).append(item)
    return list(grouped.items())


def format_boundary(boundary: float) -> str:
    """Render a bucket boundary the way Prometheus clients do.

    The `le` label is a string, and Grafana groups heatmap buckets by that
    string, so `0.005` and `0.0050` would be two different buckets. The Go
    client trims a whole-number boundary to `1` rather than `1.0`, and the
    heatmap axis reads better when this agrees with it.
    """
    if boundary == int(boundary):
        return str(int(boundary))
    return repr(boundary)


def format_labels(labels: dict[str, str]) -> str:
    if not labels:
        return ""
    pairs = ",".join(
        f'{name}="{escape_label(value)}"' for name, value in labels.items()
    )
    return "{" + pairs + "}"


def escape_label(value: str) -> str:
    return value.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n")


class MetricsHandler(BaseHTTPRequestHandler):
    instance: Instance

    def do_GET(self) -> None:  # noqa: N802 - name fixed by BaseHTTPRequestHandler
        if self.path.split("?")[0] not in ("/", "/metrics"):
            self.send_error(404)
            return
        body = self.instance.render().encode()
        self.send_response(200)
        self.send_header("Content-Type", "text/plain; version=0.0.4; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *_args: object) -> None:
        """Silence per-scrape logging: at 5s intervals it buries everything."""


def serve(instance: Instance) -> None:
    handler = type("Handler", (MetricsHandler,), {"instance": instance})
    ThreadingHTTPServer(("0.0.0.0", instance.port), handler).serve_forever()


def main() -> int:
    # "9101:app,9102:app,..." - the port is what Prometheus records as the
    # `instance` label, so this mapping is also what decides which instances
    # show up in the `instance` variable of the `variables` example.
    spec = os.environ.get(
        "EXPORTER_TARGETS",
        "9101:app,9102:app,9103:geoip,9104:geoip,9105:checkout",
    )
    started_at = time.time()
    instances = []
    for entry in spec.split(","):
        port, _, role = entry.partition(":")
        if role not in BUILDERS:
            print(f"unknown role {role!r} in EXPORTER_TARGETS", file=sys.stderr)
            return 2
        instances.append(Instance(int(port), role, started_at))

    for instance in instances:
        threading.Thread(target=serve, args=(instance,), daemon=True).start()
        print(f"serving {instance.role} metrics on :{instance.port}", file=sys.stderr)

    try:
        while True:
            time.sleep(3600)
    except KeyboardInterrupt:
        return 0


if __name__ == "__main__":
    raise SystemExit(main())
