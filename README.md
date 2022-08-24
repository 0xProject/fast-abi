# fast-abi

Encodes and decodes abi data, fast.

### Usage

```typescript
const RUST_ENCODER = new FastABI(ABI as MethodAbi[]);
const callData = RUST_ENCODER.encodeInput('sampleSellsFromUniswapV2', [...values]);
// 0x.....

// Decode the output of a method call
const output = RUST_ENCODER.decodeOutput('sampleSellsFromUniswapV2', callData);
// {
//   router: '0x6b175474e89094c44da98b954eedeac495271d0f',
//   path: [ '0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee' ],
//   takerTokenAmounts: [ 1, 2, 3 ]
// }
```

### Perf

```
    Uniswap ABI
      13 input encode
        ✓ ZeroEx - optimized (2579ms)
        p25: 0.223583ms, p50: 0.22825ms, p99: 1.734083ms, p100: 5.321458ms
        ✓ ZeroEx - no optimize (893ms)
        p25: 0.077166ms, p50: 0.077792ms, p99: 0.098417ms, p100: 2.595542ms
        ✓ fast-abi (335ms)
        p25: 0.027958ms, p50: 0.028292ms, p99: 0.032583ms, p100: 2.886333ms

      13 input decode
        ✓ ZeroEx (1399ms)
        p25: 0.112125ms, p50: 0.113542ms, p99: 0.219583ms, p100: 63.167208ms
        ✓ fast-abi (392ms)
        p25: 0.02975ms, p50: 0.030708ms, p99: 0.046667ms, p100: 4.391958ms

      13 output decode
        ✓ ZeroEx (1138ms)
        p25: 0.100583ms, p50: 0.101458ms, p99: 0.126875ms, p100: 4.380667ms
        ✓ fast-abi (327ms)
        p25: 0.023709ms, p50: 0.024625ms, p99: 0.049875ms, p100: 8.044291ms

      130 input encode
        ✓ ZeroEx - optimized (15317ms)
        p25: 1.374375ms, p50: 1.398958ms, p99: 3.235333ms, p100: 9.725875ms
        ✓ ZeroEx - no optimize (6272ms)
        p25: 0.552209ms, p50: 0.560416ms, p99: 1.711958ms, p100: 3.264542ms
        ✓ fast-abi (1569ms)
        p25: 0.140291ms, p50: 0.142292ms, p99: 0.1875ms, p100: 10.919333ms

      130 input decode
        ✓ ZeroEx (10277ms)
        p25: 0.8775ms, p50: 0.891917ms, p99: 3.149708ms, p100: 211.279542ms
        ✓ fast-abi (2381ms)
        p25: 0.189791ms, p50: 0.192541ms, p99: 0.367417ms, p100: 10.084375ms

      130 output decode
        ✓ ZeroEx (9743ms)
        p25: 0.872875ms, p50: 0.886042ms, p99: 3.282917ms, p100: 6.403833ms
        ✓ fast-abi (2244ms)
        p25: 0.177959ms, p50: 0.183959ms, p99: 0.230166ms, p100: 6.963125ms
```

### How to Publish

#### npm

```
yarn publish --access public
```

#### Rust Binary

NOTE: make sure `package.json` with a new npm package version is merged.

Push an empty commit with message `[publish binary]`. This will trigger a GitHub action step `publish` which will publish the rust binary.
