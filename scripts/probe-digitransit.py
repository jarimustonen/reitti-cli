#!/usr/bin/env python3
"""Capture and validate secret-safe Digitransit contract fixtures.

The subscription key is read only from DIGITRANSIT_SUBSCRIPTION_KEY and sent only
in the digitransit-subscription-key request header. It is never placed in a URL,
fixture, diagnostic, or command-line argument.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import re
import sys
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Any, Callable

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_FIXTURES = ROOT / "tests" / "fixtures" / "digitransit"
ROUTING = "https://api.digitransit.fi/routing/v2/hsl/gtfs/v1"
GEOCODING = "https://api.digitransit.fi/geocoding/v1"
USER_AGENT = "reitti-contract-probe/1"


def compact_features(body: Any, limit: int = 3) -> Any:
    if not isinstance(body, dict) or not isinstance(body.get("features"), list):
        return body
    result = dict(body)
    result["features"] = body["features"][:limit]
    return result


def compact_alerts(body: Any) -> Any:
    try:
        result = {"data": {"alerts": body["data"]["alerts"][:2]}}
    except (KeyError, TypeError):
        return body
    return result


def http_request(
    url: str,
    key: str | None,
    *,
    body: bytes | None = None,
    content_type: str | None = None,
    extra_headers: dict[str, str] | None = None,
) -> tuple[int, dict[str, str], bytes]:
    if "digitransit-subscription-key" in url.lower():
        raise RuntimeError("refusing a URL containing the subscription-key name")
    headers = {"User-Agent": USER_AGENT}
    if key is not None:
        headers["digitransit-subscription-key"] = key
    if content_type:
        headers["Content-Type"] = content_type
    if extra_headers:
        headers.update(extra_headers)
    request = urllib.request.Request(
        url, data=body, headers=headers, method="POST" if body is not None else "GET"
    )
    try:
        with urllib.request.urlopen(request, timeout=45) as response:
            return response.status, dict(response.headers.items()), response.read()
    except urllib.error.HTTPError as error:
        return error.code, dict(error.headers.items()), error.read()


def graphql(key: str, query: str) -> tuple[int, dict[str, str], bytes]:
    payload = json.dumps({"query": query}, separators=(",", ":")).encode()
    return http_request(
        ROUTING,
        key,
        body=payload,
        content_type="application/json",
        extra_headers={"Accept-Language": "en"},
    )


def geocode(key: str, operation: str, parameters: dict[str, str]) -> tuple[int, dict[str, str], bytes]:
    url = f"{GEOCODING}/{operation}?{urllib.parse.urlencode(parameters)}"
    return http_request(url, key)


def parse_json(data: bytes) -> Any:
    return json.loads(data.decode("utf-8"))


def selected_headers(headers: dict[str, str]) -> dict[str, str]:
    allowed = {
        "content-type",
        "content-range",
        "cache-control",
        "last-modified",
        "retry-after",
        "ratelimit-limit",
        "ratelimit-remaining",
        "x-ratelimit-limit",
        "x-ratelimit-remaining",
        "errorreason",
        "errormessage",
    }
    return {key.lower(): value for key, value in headers.items() if key.lower() in allowed}


def write_fixture(
    directory: Path,
    name: str,
    *,
    case: str,
    method: str,
    url: str,
    status: int,
    headers: dict[str, str],
    response: Any,
    retrieved_at: str,
) -> None:
    fixture = {
        "fixture_schema_version": 1,
        "case": case,
        "retrieved_at": retrieved_at,
        "request": {
            "method": method,
            "url": url,
            "authentication": "digitransit-subscription-key header from environment",
        },
        "response": {
            "status": status,
            "headers": selected_headers(headers),
            "body": response,
        },
    }
    encoded = json.dumps(fixture, ensure_ascii=False, indent=2, sort_keys=True) + "\n"
    directory.mkdir(parents=True, exist_ok=True)
    (directory / name).write_text(encoded, encoding="utf-8")


def run_live(directory: Path) -> None:
    key = os.environ.get("DIGITRANSIT_SUBSCRIPTION_KEY")
    if not key:
        raise RuntimeError(
            "DIGITRANSIT_SUBSCRIPTION_KEY is required for live probes; export it without putting it on the command line"
        )
    if any(character.isspace() for character in key):
        raise RuntimeError("DIGITRANSIT_SUBSCRIPTION_KEY contains whitespace")

    retrieved_at = dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")
    cases: list[tuple[str, str, str, Callable[[], tuple[int, dict[str, str], bytes]], Callable[[Any], Any], int]] = [
        (
            "routing-routes.json",
            "successful route lookup",
            ROUTING,
            lambda: graphql(key, '{ routes(name:"5", transportModes:[TRAM]) { gtfsId shortName longName mode } }'),
            lambda value: value,
            200,
        ),
        (
            "routing-journey.json",
            "successful itinerary planning with scheduled and estimated times",
            ROUTING,
            lambda: graphql(key, '{ planConnection(origin:{location:{coordinate:{latitude:60.168992,longitude:24.932366}}} destination:{location:{coordinate:{latitude:60.175294,longitude:24.684855}}} first:2) { pageInfo { endCursor } edges { node { start end legs { from { name } to { name } mode duration realtimeState start { scheduledTime estimated { time delay } } end { scheduledTime estimated { time delay } } route { gtfsId shortName } } } } } }'),
            lambda value: value,
            200,
        ),
        (
            "routing-stop.json",
            "successful stop lookup",
            ROUTING,
            lambda: graphql(key, '{ stop(id:"HSL:1020453") { gtfsId name lat lon wheelchairBoarding } }'),
            lambda value: value,
            200,
        ),
        (
            "routing-departures.json",
            "successful stop departures with realtime state",
            ROUTING,
            lambda: graphql(key, '{ stop(id:"HSL:1020453") { gtfsId name stoptimesWithoutPatterns(numberOfDepartures:3) { scheduledDeparture realtimeDeparture realtime realtimeState serviceDay headsign trip { gtfsId route { shortName } } } } }'),
            lambda value: value,
            200,
        ),
        (
            "routing-alerts.json",
            "current HSL service alerts",
            ROUTING,
            lambda: graphql(key, '{ alerts(feeds:["HSL"]) { feed alertHeaderText alertDescriptionText alertSeverityLevel alertEffect effectiveStartDate effectiveEndDate entities { __typename ... on Route { gtfsId } ... on Stop { gtfsId } } } }'),
            compact_alerts,
            200,
        ),
        (
            "routing-cancellations.json",
            "current realtime trip cancellations (may truthfully be empty)",
            ROUTING,
            lambda: graphql(key, '{ canceledTrips(first:5) { pageInfo { hasNextPage endCursor } edges { node { serviceDate trip { gtfsId route { shortName } } } } } }'),
            lambda value: value,
            200,
        ),
        (
            "routing-invalid-query.json",
            "GraphQL validation failure returned in a successful HTTP response",
            ROUTING,
            lambda: graphql(key, "{ stop(id:) { name } }"),
            lambda value: value,
            200,
        ),
        (
            "routing-no-result.json",
            "valid stop lookup with no matching identifier",
            ROUTING,
            lambda: graphql(key, '{ stop(id:"HSL:DOES_NOT_EXIST") { gtfsId name } }'),
            lambda value: value,
            200,
        ),
        (
            "geocoding-ambiguity.json",
            "ambiguous place search with multiple candidates",
            f"{GEOCODING}/search",
            lambda: geocode(key, "search", {"text": "Kamppi", "boundary.rect.min_lon": "24.7", "boundary.rect.min_lat": "60.1", "boundary.rect.max_lon": "25.2", "boundary.rect.max_lat": "60.35", "size": "5"}),
            compact_features,
            200,
        ),
        (
            "geocoding-autocomplete.json",
            "successful Pelias autocomplete",
            f"{GEOCODING}/autocomplete",
            lambda: geocode(key, "autocomplete", {"text": "Kamppi"}),
            compact_features,
            200,
        ),
        (
            "geocoding-place.json",
            "successful stable Pelias place lookup",
            f"{GEOCODING}/place",
            lambda: geocode(key, "place", {"ids": "gtfshsl:station:GTFS:HSL:1000102"}),
            compact_features,
            200,
        ),
        (
            "geocoding-reverse.json",
            "successful Pelias reverse geocoding",
            f"{GEOCODING}/reverse",
            lambda: geocode(key, "reverse", {"point.lat": "60.1699", "point.lon": "24.9384", "size": "1"}),
            compact_features,
            200,
        ),
        (
            "geocoding-no-result.json",
            "valid geocoding search with no candidates",
            f"{GEOCODING}/search",
            lambda: geocode(key, "search", {"text": "zzzz-no-such-place-987654321", "size": "3"}),
            compact_features,
            200,
        ),
        (
            "geocoding-invalid-place.json",
            "invalid Pelias place identifier",
            f"{GEOCODING}/place",
            lambda: geocode(key, "place", {"ids": "fictional:invalid"}),
            lambda value: value,
            400,
        ),
        (
            "routing-v1-removed.json",
            "removed Routing v1 journey-planning endpoint",
            "https://api.digitransit.fi/routing/v1/routers/hsl/index/graphql",
            lambda: http_request(
                "https://api.digitransit.fi/routing/v1/routers/hsl/index/graphql",
                key,
                body=json.dumps({"query": "{ routes { gtfsId } }"}).encode(),
                content_type="application/json",
            ),
            lambda value: value,
            404,
        ),
        (
            "routing-data-v3.json",
            "Routing Data v3 artifact index",
            "https://api.digitransit.fi/routing-data/v3/hsl/",
            lambda: http_request("https://api.digitransit.fi/routing-data/v3/hsl/", key),
            lambda value: value,
            200,
        ),
    ]

    for filename, case, url, request, transform, expected_status in cases:
        status, headers, raw = request()
        if status != expected_status:
            raise RuntimeError(f"{case}: expected HTTP {expected_status}, received {status}")
        if key.encode() in raw:
            raise RuntimeError(f"{case}: response unexpectedly contained the subscription key")
        if filename == "routing-v1-removed.json":
            parsed: Any = raw.decode("utf-8", errors="replace")
        elif filename == "routing-data-v3.json":
            text = raw.decode("utf-8", errors="replace")
            parsed = {
                "artifact_names": sorted(set(re.findall(r'href="([^"]+(?:\.zip|\.obj|\.json|\.pbf|\.tif|version\.txt|build\.log))"', text)))[:30]
            }
            if not parsed["artifact_names"]:
                raise RuntimeError("Routing Data v3 index did not expose recognizable artifact names")
        else:
            parsed = parse_json(raw)
        write_fixture(
            directory,
            filename,
            case=case,
            method="POST" if url == ROUTING or "routing/v1/" in url else "GET",
            url=url,
            status=status,
            headers=headers,
            response=transform(parsed),
            retrieved_at=retrieved_at,
        )
        print(f"OK {case}: HTTP {status}")

    for name, url, cache_max_age in [
        ("service_alerts", "https://realtime.hsl.fi/realtime/service-alerts/v2/hsl", 5),
        ("trip_updates", "https://realtime.hsl.fi/realtime/trip-updates/v2/hsl", 5),
        ("vehicle_positions", "https://realtime.hsl.fi/realtime/vehicle-positions/v2/hsl", 1),
    ]:
        status, headers, raw = http_request(url, None, extra_headers={"Range": "bytes=0-31"})
        if status != 206 or headers.get("Content-Type") != "application/x-protobuf":
            raise RuntimeError(f"HSL GTFS-RT {name}: expected HTTP 206 application/x-protobuf")
        if key.encode() in raw:
            raise RuntimeError(f"HSL GTFS-RT {name}: response unexpectedly contained the subscription key")
        metadata = {
            "feed": name,
            "url": url,
            "status": status,
            "content_type": headers.get("Content-Type"),
            "content_range": headers.get("Content-Range"),
            "cache_control": headers.get("Cache-Control"),
            "last_modified": headers.get("Last-Modified"),
            "documented_max_age_seconds": cache_max_age,
            "sampled_bytes": len(raw),
        }
        write_fixture(
            directory,
            f"hsl-gtfs-rt-{name}.json",
            case=f"HSL GTFS-RT {name} metadata and protobuf signature",
            method="GET",
            url=url,
            status=status,
            headers=headers,
            response=metadata,
            retrieved_at=retrieved_at,
        )
        print(f"OK HSL GTFS-RT {name}: HTTP {status}")

    check_fixtures(directory, key)


def check_fixtures(directory: Path, active_key: str | None = None) -> None:
    expected = {
        "routing-routes.json",
        "routing-journey.json",
        "routing-stop.json",
        "routing-departures.json",
        "routing-alerts.json",
        "routing-cancellations.json",
        "routing-invalid-query.json",
        "routing-no-result.json",
        "geocoding-ambiguity.json",
        "geocoding-autocomplete.json",
        "geocoding-place.json",
        "geocoding-reverse.json",
        "geocoding-no-result.json",
        "geocoding-invalid-place.json",
        "routing-v1-removed.json",
        "routing-data-v3.json",
        "hsl-gtfs-rt-service_alerts.json",
        "hsl-gtfs-rt-trip_updates.json",
        "hsl-gtfs-rt-vehicle_positions.json",
    }
    missing = sorted(name for name in expected if not (directory / name).is_file())
    if missing:
        raise RuntimeError(f"missing fixtures: {', '.join(missing)}")
    for path in sorted(directory.glob("*.json")):
        raw = path.read_bytes()
        fixture = json.loads(raw)
        url = fixture.get("request", {}).get("url", "")
        if "digitransit-subscription-key" in url.lower():
            raise RuntimeError(f"{path}: secret header name appears in URL")
        if active_key and active_key.encode() in raw:
            raise RuntimeError(f"{path}: active subscription key found")
        if fixture.get("fixture_schema_version") != 1:
            raise RuntimeError(f"{path}: unsupported fixture schema")
    print(f"OK {len(list(directory.glob('*.json')))} fixtures are valid and URL-safe")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("live", "check"))
    parser.add_argument("--output-dir", type=Path, default=DEFAULT_FIXTURES)
    args = parser.parse_args()
    try:
        if args.command == "live":
            run_live(args.output_dir)
        else:
            check_fixtures(args.output_dir, os.environ.get("DIGITRANSIT_SUBSCRIPTION_KEY"))
    except (OSError, ValueError, RuntimeError, json.JSONDecodeError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
