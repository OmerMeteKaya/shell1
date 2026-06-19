# Zesh

A modern shell written in Rust.

## Build

    make          # release build → zesh-rs/target/release/zesh_rs
    make debug    # debug build
    make test     # build + run test suite (78 PASS 0 FAIL 2 SKIP)
    make install  # install to /usr/local/bin/zesh

## Configuration

    ~/.zesh/config.toml   — generated on first run with defaults

## Validation

✓ make build           → exit 0
✓ make test            → 78 PASS 0 FAIL 2 SKIP

## License

This project is licensed under the GNU General Public License v3.0 - see the [LICENSE](LICENSE) file for details.
