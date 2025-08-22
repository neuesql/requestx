# HTTP Client Performance Optimization Strategies

## Master Strategy List

### Connection Management
- [x] **pool-max-idle-per-host**: Optimize idle connection limits per host
- [x] **pool-idle-timeout**: Adjust connection timeout settings for sustained load
- [ ] **pool-max-lifetime**: Configure connection lifecycle for long-running benchmarks

### Request Processing
- [ ] **buffer-size**: Optimize buffer allocation for single requests and high concurrency
- [ ] **compression**: Test compression settings with large payloads
- [ ] **keep-alive**: Configure keep-alive for multiple sequential requests

### Memory Management
- [ ] **allocation-patterns**: Monitor memory patterns with low concurrency, high request counts
- [ ] **buffer-reuse**: Implement buffer reuse strategies for repeated requests
- [ ] **garbage-collection**: Tune async runtime garbage collection

### Concurrency & Async Runtime
- [ ] **thread-pool-sizing**: Optimize thread pool for high concurrency
- [ ] **async-runtime**: Configure async runtime parameters
- [ ] **lock-contention**: Reduce lock contention under high load

### Network & Protocol
- [ ] **tcp-socket-options**: Optimize TCP socket options for various network conditions
- [ ] **http-version**: Compare HTTP/2 vs HTTP/1.1 performance
- [ ] **connection-warmup**: Test connection warmup strategies for cold start

## Current Status
- **Next Strategy**: http2-initial-stream-window-size
- **Branch**: refactor/optimization-http2-initial-stream-window-size (to be created)
- **Target**: Optimize HTTP/2 stream window size for throughput vs memory trade-off
- **Expected Improvement**: 5-10% memory efficiency improvement
- **Test Parameters**: --concurrency 1024 --requests 5000

## Completed Strategies
- [x] **pool-max-idle-per-host**: Reduced from 2048 to 1024 connections
  - **Status**: ⚠️ Mixed Results (0.91% memory improvement, 8.07% CPU increase)
  - **Recommendation**: Consider for memory-constrained environments
  - **Branch**: refactor/optimization-pool-max-idle-per-host (completed)

- [x] **pool-idle-timeout**: Reduced from 90 to 30 seconds
  - **Status**: ⚠️ Minimal Results (0.09% memory improvement, 3.3% CPU increase)
  - **Recommendation**: Consider for memory-constrained environments, test lower values
  - **Branch**: refactor/optimization-pool-idle-timeout (completed)