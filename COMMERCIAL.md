# Commercial support & sponsorship

IZANAGI is free and open source under its current licenses — see
[`LICENSE`](./LICENSE) — and stays that way. There is no paid feature gate:
the crates on crates.io are the full product. Revenue funds maintenance,
and the model is deliberately simple: the software is free, the maintainer's
time is not.

## Ways to pay

| Channel | What it buys | Where |
| --- | --- | --- |
| Paid-support funnel | Ongoing maintenance of both crates — the link in the repository's Sponsor slot (set by [`.github/FUNDING.yml`](./.github/FUNDING.yml)) opens the commercial-support issue form | [Open a commercial-support request](https://github.com/shizukutanaka/IZANAGI/issues/new?template=commercial_support.yml) |
| Commercial support | Priority fixes, integration help for `izanagi`/`izanagi_kit`, custom detector or format modules, determinism-review of a shipping simulation | The issue form — or any issue titled `[commercial] …` |
| Contract development | Scoped work beyond open-source maintenance (new modules, engine-side features, migration help) | Same — describe scope in the issue |

## Rules of engagement

- **Nothing is paywalled.** A request that benefits the product (bug fix,
  missing module, better detection) lands in the public repository for
  everyone; payment buys priority and dedicated effort, not exclusivity.
- **The license does not change.** Companies already may use both crates
  commercially under MIT / MIT OR Apache-2.0. What a company *cannot* get
  for free is guaranteed response time and custom work — that is the paid
  surface.
- **No telemetry, ads, or nagging.** The crates make no network calls and
  never will for monetization purposes; the only revenue surfaces are this
  document, the FUNDING.yml link, and the issue form. Payment is confirmed
  manually out-of-band — nothing in the repository processes money.

## For companies evaluating IZANAGI

The crates carry no runtime dependencies and forbid `unsafe` code, so the
audit surface is small. Support engagements typically cover: proving a
simulation replays bit-exactly across the platforms a studio ships on,
adding a file-format detector a team needs, or hardening an integration
before release.
