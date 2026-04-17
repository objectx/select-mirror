# select-mirror

Probes a list of Ubuntu mirror URLs in parallel and prints the fastest one to stdout.

## Usage

```
select-mirror [OPTIONS] <MIRRORS>...

Arguments:
  <MIRRORS>...  One or more mirror base URLs to probe

Options:
      --probe-path <PROBE_PATH>        Path appended to each mirror URL [default: /ubuntu/dists/noble/Release]
      --timeout <TIMEOUT>              Request timeout in seconds [default: 3]
      --fast-threshold <FAST_THRESHOLD>
                                       Response-time threshold in milliseconds to qualify as "fast" (e.g. 500 = 0.5 s) [default: 500]
      --fast-count <FAST_COUNT>        Stop after this many mirrors respond within --fast-threshold [default: 3]
  -h, --help                           Print help
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

## Build

```bash
cargo build --release
# Binary: target/release/select-mirror
```

## Reference

The original shell implementation is in [`reference/select-mirror.sh`](reference/select-mirror.sh).
