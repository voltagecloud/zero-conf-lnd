# Zero Conf LND

This program runs as a daemon and connects to your own LND to accept zero conf channels from configured whitelisted nodes.

## Running

### Setup

In order for this to work, there's two flags that are needed in LND:

```
protocol.option-scid-alias=true
protocol.zero-conf=true
```

After setting this up, this program needs to run in order to accept channels in general (even non-zero-conf channels). 

### Start Program

To run this program, setup the config file and run with it:

```
./zero-conf-lnd config.yml
```

You may take a look at `example.config.yml` for how to set this up.

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
