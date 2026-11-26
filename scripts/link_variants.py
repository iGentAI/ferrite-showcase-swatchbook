#!/usr/bin/env python3
"""Set up the "same data, two designs" pair over Swatchbook's own HTTP API.

    link_variants.py claim URL --key-file F
        Set the admin key of a freshly deployed app (only works while none is set).
        Do this on Design A BEFORE forking: the fork copies the key's hash.

    link_variants.py link --a URL_A --b URL_B --key-file F [--b-preset poster]
        Restyle B from a built-in preset, label the two variants "Design A" and
        "Design B", point each at the other, and start a fresh scoreboard round
        on both so the comparison begins at zero.

The admin key is read from a file and never printed. Standard library only.
"""

import argparse
import json
import pathlib
import sys
import urllib.error
import urllib.request


def call(base, method, path, body=None):
    data = None if body is None else json.dumps(body).encode()
    request = urllib.request.Request(
        base.rstrip("/") + path,
        data=data,
        method=method,
        headers={"content-type": "application/json", "accept": "application/json"},
    )
    try:
        with urllib.request.urlopen(request, timeout=60) as response:
            return response.status, json.loads(response.read() or b"null")
    except urllib.error.HTTPError as e:
        try:
            payload = json.loads(e.read() or b"null")
        except ValueError:
            payload = None
        return e.code, payload


def must(label, result, ok=(200, 201)):
    status, payload = result
    if status not in ok:
        sys.exit(f"{label} failed: HTTP {status} {payload}")
    print(f"{label}: HTTP {status}")
    return payload


def set_site(base, key, label, sibling):
    current = must(f"read site on {base}", call(base, "GET", "/api/site"))["site"]
    current.update({"label": label, "sibling_url": sibling})
    must(f"label {base} as {label!r}", call(base, "POST", "/api/site", {"admin_key": key, "site": current}))


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    p = sub.add_parser("claim")
    p.add_argument("url")
    p.add_argument("--key-file", required=True)
    p = sub.add_parser("link")
    p.add_argument("--a", required=True, help="URL of Design A (the original)")
    p.add_argument("--b", required=True, help="URL of Design B (the fork)")
    p.add_argument("--key-file", required=True)
    p.add_argument("--b-preset", default="poster", help="built-in preset for Design B (gallery, poster, soft)")
    p.add_argument("--keep-scores", action="store_true", help="do not start a fresh scoreboard round")
    args = parser.parse_args()
    key = pathlib.Path(args.key_file).read_text().strip()

    if args.command == "claim":
        status, payload = call(args.url, "POST", "/api/admin/claim", {"admin_key": key})
        if status == 409:
            must("an admin key is already set; checking it is ours", call(args.url, "POST", "/api/admin/check", {"admin_key": key}))
        else:
            must("claim admin key", (status, payload))
        return 0

    a, b = args.a.rstrip("/"), args.b.rstrip("/")
    for base in (a, b):
        must(f"admin key accepted by {base}", call(base, "POST", "/api/admin/check", {"admin_key": key}))
    catalog = must("read the preset catalog", call(b, "GET", "/api/theme"))["catalog"]
    preset = catalog["presets"].get(args.b_preset)
    if preset is None:
        sys.exit(f"unknown preset {args.b_preset!r}; choose from {sorted(catalog['presets'])}")
    must(f"restyle Design B as {preset['name']!r}", call(b, "POST", "/api/theme", {"admin_key": key, "theme": preset}))
    set_site(a, key, "Design A", b)
    set_site(b, key, "Design B", a)
    if not args.keep_scores:
        for base in (a, b):
            payload = must(f"fresh scoreboard on {base}", call(base, "POST", "/api/admin/reset-scores", {"admin_key": key}))
            print(f"  round {payload['round']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
