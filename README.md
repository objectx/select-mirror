# select-mirror

Probes a list of Ubuntu mirror URLs in parallel and prints the fastest one to stdout.

## Usage

```
select-mirror [OPTIONS] <MIRRORS>...

Arguments:
  <MIRRORS>...  One or more mirror base URLs to probe

Options:
      --probe-path <PROBE_PATH>  Path appended to each mirror URL [default: /ubuntu/dists/noble/Release]
      --timeout <TIMEOUT>        Request timeout in seconds [default: 3]
  -h, --help                     Print help
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

## Build

```bash
cargo build --release
# Binary: target/release/select-mirror
```

## Reference

The original shell implementation is in [`reference/select-mirror.sh`](reference/select-mirror.sh).
