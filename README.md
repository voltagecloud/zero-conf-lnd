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
- libssl-dev

## Zeroconf Probing

The plugin includes the responder side of an ad-hoc protocol based on `custommsg`s that a funder node can use to ask a fundee node whether or not it will accept zeroconf channels from the funder node.

This ad-hoc protocol is a JSON-RPC 2.0-like request-response protocol on top of `custommsg`s, whereby all the bytes following the 2-byte type prefix form a valid UTF-8 JSON encoding. Essentially:

```py3
message_bytes = <type>.to_bytes(2, 'big') + json.dumps(<payload>).encode('utf-8')
```

### For a funder to probe whether a fundee accepts zeroconf channels from the funder

> Note: this part is NOT included in the plugin
1. Generate a uuid4 string (call it `uuid`)
2. Send the peer a `custommsg` like this (encode into `message_bytes` using above):
    
    type(u16, 2-byte): `55443`
    payload:
    ```json
    {
        "id": <uuid>,
        "method": "getzeroconfinfo"
    }
    ```

### For a fundee to respond to the zeroconf probing request

> Note: this part IS included in the plugin
1. Get the request's uuid4 id (call it `uuid`)
2. Send the requester a `custommsg` like this (encode into `message_bytes` using above):
    
    type(u16, 2-byte): `55445`
    payload:
    ```json
    {
        "id": <uuid>,
        "result": {
            "allows_your_zeroconf": true  # or false if not allowed
        }
    }
    ```
