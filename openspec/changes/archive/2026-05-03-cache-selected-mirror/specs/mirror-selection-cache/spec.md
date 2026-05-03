## ADDED Requirements

### Requirement: Cache file location

The system SHALL persist the most recently selected mirror to a JSON file. By default the file SHALL be `.selected-mirror.json` in the current working directory. The system SHALL accept a `--cache-file <path>` flag to override the location with an arbitrary path.

#### Scenario: Default cache file path

- **WHEN** the user runs `select-mirror <mirrors...>` without `--cache-file`
- **THEN** the system reads from and writes to `.selected-mirror.json` in the current working directory

#### Scenario: Override cache file path

- **WHEN** the user runs `select-mirror --cache-file /tmp/mycache.json <mirrors...>`
- **THEN** the system reads from and writes to `/tmp/mycache.json`

### Requirement: Cache file schema

The cache file SHALL be a JSON document containing exactly these fields: `version` (integer), `mirror` (string), `elapsed_ms` (integer), `probe_path` (string), `recorded_at` (UNIX epoch seconds, u64). The system SHALL treat any cache file with an unrecognized `version` as a cache miss.

#### Scenario: Well-formed cache is parsed

- **WHEN** the cache file contains valid JSON with all required fields and a recognized `version`
- **THEN** the system uses it for the cache-hit gate

#### Scenario: Unknown version is ignored

- **WHEN** the cache file's `version` field is not recognized by the running binary
- **THEN** the system treats it as a cache miss and proceeds to probe all mirrors

#### Scenario: Malformed JSON is ignored

- **WHEN** the cache file exists but does not parse as valid JSON, or is missing required fields
- **THEN** the system treats it as a cache miss without aborting

### Requirement: Cache validation gate

Before using a cache entry, the system SHALL verify ALL of the following conditions. If any condition fails, the cache entry SHALL be treated as a miss:

1. The cache file exists and parses successfully.
2. The cache file's `version` field is recognized.
3. The cache file's `probe_path` field equals the current `--probe-path`.
4. The cache file's `mirror` field matches one of the entries in the current `mirrors` argv.

#### Scenario: All conditions pass

- **WHEN** all four conditions are satisfied
- **THEN** the system proceeds to probe the cached mirror as a hit-candidate

#### Scenario: Probe-path mismatch invalidates cache

- **WHEN** the cached `probe_path` differs from the current `--probe-path`
- **THEN** the system treats the cache as a miss

#### Scenario: Cached mirror absent from argv

- **WHEN** the cached `mirror` is not in the current `mirrors` argv list
- **THEN** the system treats the cache as a miss

#### Scenario: Cache file does not exist

- **WHEN** no cache file is present at the configured path
- **THEN** the system treats the cache as a miss without erroring

### Requirement: Cache hit short-circuit

When the cache validation gate passes, the system SHALL probe ONLY the cached mirror with the configured `--timeout`. If the probe succeeds with elapsed time strictly less than `--fast-threshold`, the system SHALL print the cached mirror to stdout, update the cache file with the new measurement, and exit successfully without probing any other mirror.

#### Scenario: Cached mirror responds within threshold

- **WHEN** the cached mirror responds with elapsed time less than `--fast-threshold`
- **THEN** the system prints only the cached mirror URL to stdout
- **AND** does not probe any other mirror
- **AND** updates the cache file with the new `elapsed_ms` and `recorded_at`

#### Scenario: Cached mirror responds but exceeds threshold

- **WHEN** the cached mirror responds with elapsed time at or above `--fast-threshold`
- **THEN** the system falls through to the probe-all flow

#### Scenario: Cached mirror probe fails

- **WHEN** the cached mirror probe fails or times out
- **THEN** the system falls through to the probe-all flow

### Requirement: Stderr observability on cache hit

When the cache short-circuit fires, the system SHALL print one informational line to stderr identifying the cached mirror, the measured elapsed time, and a `(cached)` marker.

#### Scenario: Cache hit announces itself

- **WHEN** the system uses a cache hit
- **THEN** stderr contains a line of the form `<mirror_url>: <elapsed>s (cached)` for the chosen mirror

### Requirement: Cache miss falls through to probe-all

When the cache is a miss for any reason — invalid file, failed validation gate, slow probe, or failed probe — the system SHALL run the existing parallel probe-all flow with all mirrors from argv, including the cached mirror if it is in argv.

#### Scenario: Cache miss probes all mirrors in parallel

- **WHEN** the cache validation gate fails
- **THEN** the system probes every mirror in the argv list using the existing parallel-probe logic, including any early-exit by `--fast-count` and `--fast-threshold`

#### Scenario: Cached probe was slow, falls through

- **WHEN** the cached mirror responded but at or above `--fast-threshold`
- **THEN** the system probes every mirror in the argv list, including the cached one (no deduplication)

### Requirement: Cache write on every successful selection

After a successful selection — whether via cache hit or via probe-all — the system SHALL write the chosen mirror, its measured elapsed time, the current `--probe-path`, the current schema `version`, and the current UTC timestamp to the cache file. The write SHALL be atomic via `tmp + rename`.

#### Scenario: Cache hit updates the cache file

- **WHEN** a cache hit fires and the system exits successfully
- **THEN** the cache file at the configured path contains the cached mirror with the freshly measured `elapsed_ms` and an updated `recorded_at`

#### Scenario: Cache miss followed by probe-all updates the cache file

- **WHEN** the system selects a winner via the probe-all flow
- **THEN** the cache file at the configured path contains the new winner's URL, elapsed time, and current `recorded_at`

#### Scenario: Atomic write on concurrent invocations

- **WHEN** multiple `select-mirror` invocations terminate concurrently in the same directory
- **THEN** the cache file at the configured path is always either the prior valid content or one of the freshly written valid contents — never partially written or corrupt

### Requirement: Cache write failures are non-fatal

If writing the cache file fails (e.g., permissions, full disk, read-only filesystem), the system SHALL print a warning to stderr and exit successfully with the chosen mirror still on stdout.

#### Scenario: Cache write fails on read-only filesystem

- **WHEN** the cache file path is on a read-only filesystem and the system selects a winner
- **THEN** the system prints the chosen mirror to stdout, prints a warning to stderr, and exits with status 0

### Requirement: `--no-cache` flag

The system SHALL accept a `--no-cache` flag. When present, the flag SHALL cause the system to skip reading the cache file (treat as miss unconditionally) but SHALL NOT prevent writing the freshly chosen mirror to the cache file.

#### Scenario: `--no-cache` forces probe-all

- **WHEN** the user runs `select-mirror --no-cache <mirrors...>` and a valid cache file exists at the default path
- **THEN** the system ignores the cache file and runs the probe-all flow

#### Scenario: `--no-cache` still writes cache

- **WHEN** the user runs `select-mirror --no-cache <mirrors...>` and the probe-all flow selects a winner
- **THEN** the cache file at the configured path is updated with the new winner

### Requirement: All-mirrors-failed exit semantics preserved

When neither the cache hit path nor the probe-all path can produce a winner (cache miss and all mirrors fail), the system SHALL exit with status 1 and print the existing error message to stderr, matching the prior behavior.

#### Scenario: Cache miss and all probes fail

- **WHEN** the cache is a miss and every mirror in argv fails to respond within `--timeout`
- **THEN** the system exits with status 1 and prints `Error: all mirrors failed or timed out` to stderr
