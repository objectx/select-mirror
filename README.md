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

Use `--no-cache` to force a fresh probe while still updating the cache for the next run. Use `--cache-file /dev/null` to disable caching entirely.

## Build

```bash
cargo build --release
# Binary: target/release/select-mirror
```

## Reference

The original shell implementation is in [`reference/select-mirror.sh`](reference/select-mirror.sh).
