# Zero Conf LND

This program runs as a daemon and connects to your own LND to accept zero conf channels from configured whitelisted nodes.

## Development

First copy the example config and change the values to match your environment:
```
cp example.config.yml .local.config.yml
```

Now run with that config path as the first argument:
```
cargo run .local.config.yml
```

### Dependencies

- cmake
