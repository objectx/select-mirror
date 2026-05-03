# select-mirror

Probes a list of Ubuntu mirror URLs in parallel and prints the fastest one to stdout.

## Usage

```
select-mirror [OPTIONS] <MIRRORS>...

Arguments:
  <MIRRORS>...  One or more mirror base URLs to probe

Options:
      --probe-path <PROBE_PATH>
          Path appended to each mirror URL [default: /ubuntu/dists/noble/Release]
      --timeout <TIMEOUT>
          Request timeout in seconds [default: 3]
      --fast-threshold <FAST_THRESHOLD>
          Response-time threshold in milliseconds to qualify as "fast" [default: 500]
      --fast-count <FAST_COUNT>
          Stop after this many mirrors respond within --fast-threshold [default: 3]
      --cache-file <CACHE_FILE>
          Path to the cache file for persisting the selected mirror
          [default: .selected-mirror.json]
      --no-cache
          Skip reading the cache and re-probe all mirrors (still writes the result)
  -h, --help
          Print help
```

## Example

```bash
select-mirror \
  http://ftp.jaist.ac.jp/pub/Linux/ubuntu \
  http://ftp.iij.ad.jp/pub/linux/ubuntu/archive \
  http://jp.archive.ubuntu.com/ubuntu \
  http://archive.ubuntu.com/ubuntu
```

Output (stderr shows timings, stdout prints the winner):

```
  http://ftp.jaist.ac.jp/pub/Linux/ubuntu: 0.123s
  http://jp.archive.ubuntu.com/ubuntu: 0.251s
  http://ftp.iij.ad.jp/pub/linux/ubuntu/archive: 0.198s
  http://archive.ubuntu.com/ubuntu: 0.312s
http://ftp.jaist.ac.jp/pub/Linux/ubuntu
```

Probing stops early once `--fast-count` mirrors respond within `--fast-threshold` ms. In the example above, if the first three mirrors all respond under 500 ms the fourth probe is skipped and the fastest of the three is printed.

## Caching

On each successful run the tool writes the chosen mirror to `.selected-mirror.json` (in the current directory by default). On the next invocation it probes only that cached mirror first; if it responds within `--fast-threshold` it is returned immediately without probing any other mirror.

This makes consecutive invocations return the same mirror as long as the network is stable — useful when the output drives a Docker `apt` mirror layer that you do not want to rebuild unnecessarily.

Use `--no-cache` to force a fresh probe while still updating the cache for the next run.

## Sandboxes and proxies

`select-mirror` honours the standard proxy environment variables. When any of `ALL_PROXY`, `HTTPS_PROXY`, or `HTTP_PROXY` (or their lowercase equivalents) is set, probes are routed through the configured proxy. The first variable that parses as a `ureq` proxy URL wins; unparseable values (for example `socks5h://…`, a scheme `ureq 2.x` does not recognize) are skipped and the next variable is tried.

`NO_PROXY` is honoured with a minimal matcher: each comma-separated entry matches as either an exact hostname or a dot-suffix (`example.com` matches `api.example.com`). Leading `*.` or `.` on an entry is stripped before matching. Loopback entries such as `127.0.0.1`, `::1`, and `*.local` therefore bypass the proxy automatically when listed in `NO_PROXY`.

This makes `select-mirror` work transparently inside network-filtered sandboxes (e.g. Claude Code's `sandbox.enabled: true` mode) where outbound traffic is forced through a local HTTP proxy and direct DNS for arbitrary hosts is blocked. No flags or sandbox-specific configuration are required from the caller — the standard env-var contract is sufficient.

When none of the proxy variables are set, the tool connects directly as before.

## Build

```bash
cargo build --release
# Binary: target/release/select-mirror
```

## Reference

The original shell implementation is in [`reference/select-mirror.sh`](reference/select-mirror.sh).
