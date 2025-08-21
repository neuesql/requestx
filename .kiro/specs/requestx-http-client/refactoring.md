# Rust HTTP Client Performance Optimization Workflow

## Overview
A systematic approach to identify, implement, and measure performance improvements in Rust HTTP client code.

## Phase 1: Analysis & Planning

### 1.1 Performance Analysis
- [ ] Analyze current HTTP client implementation in `src/` folder
- [ ] Review `config.toml` configuration parameters
- [ ] Identify performance bottlenecks using profiling tools
- [ ] Document baseline performance metrics

### 1.2 Strategy Documentation
- [ ] Create `tests_refactoring/todo.md` with comprehensive strategy list
- [ ] For each strategy, create individual `tests_refactoring/optimization-{strategy-name}.md` files
- [ ] Define specific improvement targets, for example:
  - Response time improvement (e.g., 10% faster)
  - Throughput increase (higher RPS)
  - Memory usage reduction (e.g., 20% lower)
  - CPU utilization optimization
  - Connection pool efficiency
  - Memory allocation patterns
  - Async runtime performance

## Phase 2: Strategy Implementation

### 2.1 Branch Management
```bash
# Create optimization branch
git checkout -b refactor/optimization-{strategy-name}
```

### 2.2 Implementation Process
- [ ] Select one strategy from `tests_refactoring/todo.md`
- [ ] Implement changes step-by-step
- [ ] Focus on single performance factor per iteration
- [ ] Build and validate changes based on modification type:

#### For Rust Code Changes (src/ folder modifications)
```bash
make clean && make build
```

#### For Configuration Changes (config.toml only)
- No rebuild required - configuration is loaded at runtime
- Verify configuration syntax is valid
- Document configuration parameter changes

- [ ] Ensure successful validation before proceeding to benchmarking

### 2.3 Commit Standards
- [ ] Use descriptive commit messages:
  ```
  feat(perf): implement {strategy-name} optimization
  
  - {specific changes made}
  - Target: {improvement metric}
  - Affects: {components modified}
  ```

## Phase 3: Performance Measurement

### 3.1 Benchmark Execution
Run comprehensive benchmarks with strategy-specific parameters. Adjust concurrency and total requests based on optimization target. Run each strategy **at least 3 times** to establish stable average values.

**Choose appropriate library based on strategy:**
- Use `requestx-sync` for synchronous optimizations
- Use `requestx-async` for asynchronous optimizations  
- Use both for comparative analysis

```bash
# Example command - adjust parameters per strategy
uv run python scripts/requestx-benchmark.py \
  --method get \
  --libraries {requestx-sync|requestx-async|requestx-sync,requestx-async} \
  --csv \
  --db \
  --output-dir ./reports \
  --concurrency {strategy-specific-value} \
  --requests {strategy-specific-value} \
  --factor optimization-{strategy-name} \
  --reasoning "{improvement-description}" \
  --commit $(git rev-parse HEAD) \
  --branch refactor/optimization-{strategy-name}
```

### 3.2 Strategy-Specific Benchmark Guidelines

#### Connection Pool Optimizations
```bash
--concurrency 100 --requests 5000  # Test connection reuse
```

#### High Throughput Optimizations  
```bash
--concurrency 1000 --requests 10000  # Stress test performance
```

#### Memory Optimizations
```bash
--concurrency 50 --requests 1000  # Monitor memory patterns
```

#### Single Request Optimizations
```bash
--concurrency 1 --requests 1     # Single request baseline performance
--concurrency 1 --requests 128   # Medium-scale sequential processing
--concurrency 1 --requests 1024  # Large-scale single-threaded performance
--concurrency 1 --requests 2048  # Extended sequential request testing
```

### 3.3 Benchmark Execution Protocol
- [ ] Run baseline measurements before optimization
- [ ] Execute **3 benchmark runs minimum** for each configuration
- [ ] Calculate average, min, max values from multiple runs
- [ ] Document environmental conditions (system load, time of day)
- [ ] Use consistent hardware/system state for comparisons

## Phase 4: Analysis & Documentation

### 4.1 Results Comparison
- [ ] Compare new metrics against baseline
- [ ] Calculate percentage improvements/regressions
- [ ] Document findings in strategy-specific markdown file

### 4.2 Documentation Updates
- [ ] Update `tests_refactoring/optimization-{strategy-name}.md` with results
- [ ] Update `tests_refactoring/todo.md` with completion status
- [ ] Include before/after performance metrics with statistical analysis
- [ ] Document all 3+ benchmark runs and their variations

### 4.3 Result Documentation Template
Store in `tests_refactoring/optimization-{strategy-name}.md`:

```markdown
## Optimization Strategy: {strategy-name}

### Objective
- **Target Metric**: {primary improvement target}
- **Expected Improvement**: {percentage or absolute value}
- **Rationale**: {why this optimization should work}

### Implementation Details
- **Files Modified**: {list of changed files}
- **Key Changes**: {bullet points of changes made}
- **Configuration Updates**: {config.toml changes if any}
- **Commit**: {commit-hash}
- **Branch**: refactor/optimization-{strategy-name}

### Benchmark Configuration
- **Library**: {requestx-sync|requestx-async|both}
- **Concurrency**: {value}
- **Total Requests**: {value}
- **Runs Executed**: {number of benchmark runs}

### Performance Results

#### Baseline (Before Optimization)
| Metric | Run 1 | Run 2 | Run 3 | Average | Std Dev |
|--------|-------|-------|-------|---------|---------|
| Response Time (ms) | {val} | {val} | {val} | {val} | {val} |
| RPS | {val} | {val} | {val} | {val} | {val} |
| Memory (MB) | {val} | {val} | {val} | {val} | {val} |
| CPU (%) | {val} | {val} | {val} | {val} | {val} |

#### Optimized (After Implementation)
| Metric | Run 1 | Run 2 | Run 3 | Average | Std Dev |
|--------|-------|-------|-------|---------|---------|
| Response Time (ms) | {val} | {val} | {val} | {val} | {val} |
| RPS | {val} | {val} | {val} | {val} | {val} |
| Memory (MB) | {val} | {val} | {val} | {val} | {val} |
| CPU (%) | {val} | {val} | {val} | {val} | {val} |

#### Improvement Summary
- **Response Time**: {before_avg}ms → {after_avg}ms (**{percentage_change}%**)
- **RPS**: {before_avg} → {after_avg} (**{percentage_change}%**)
- **Memory Usage**: {before_avg}MB → {after_avg}MB (**{percentage_change}%**)
- **CPU Usage**: {before_avg}% → {after_avg}% (**{percentage_change}%**)

### Analysis
- **Primary Success**: {met target improvement? yes/no}
- **Side Effects**: {any unexpected changes in other metrics}
- **Stability**: {consistency across multiple runs}
- **Trade-offs**: {what was sacrificed for the improvement}

### Conclusion
- **Status**: {✅ Success | ❌ Failed | ⚠️ Mixed Results}
- **Recommendation**: {merge to main | needs further work | abandon}
- **Next Steps**: {follow-up optimizations or investigations}
```

## Phase 5: Integration

### 5.1 Successful Optimizations
If performance improvement is achieved:
- [ ] Merge branch to main:
  ```bash
  git checkout main
  git merge refactor/optimization-{strategy-name}
  git push origin main
  ```
- [ ] Use clear merge commit message:
  ```
  feat(perf): merge {strategy-name} optimization
  
  Achieved {percentage}% improvement in {metric}
  Benchmark results: {key metrics}
  ```

### 5.2 Failed Optimizations
If no improvement or regression occurs:
- [ ] Document lessons learned in `tests_refactoring/optimization-{strategy-name}.md`
- [ ] Keep branch for future reference
- [ ] Update strategy status in `tests_refactoring/todo.md`
- [ ] Consider alternative approaches or parameter tuning

## File Organization Structure

All optimization documentation should be organized under the `tests_refactoring/` directory:

```
tests_refactoring/
├── todo.md                                    # Master strategy list and status
├── optimization-pool-max-idle.md              # Individual strategy documentation
├── optimization-buffer-size.md                # Individual strategy documentation  
├── optimization-compression.md                # Individual strategy documentation
└── ...                                        # Additional strategy files
```

### File Naming Convention
- **Master list**: `tests_refactoring/todo.md`
- **Strategy files**: `tests_refactoring/optimization-{strategy-name}.md`
- **Branch names**: `refactor/optimization-{strategy-name}`

## Common Optimization Strategies

### Connection Management
- `pool_max_idle_per_host`: Optimize idle connection limits (test with concurrency 100+)
- `pool_idle_timeout`: Adjust connection timeout settings (test with sustained load)
- `pool_max_lifetime`: Configure connection lifecycle (test with long-running benchmarks)

### Request Processing
- Buffer size optimization (test with single requests and high concurrency)
- Compression settings (test with large payloads)
- Keep-alive configuration (test with multiple sequential requests)

### Memory Management  
- Allocation patterns (monitor with low concurrency, high request counts)
- Buffer reuse strategies (test with repeated requests)
- Garbage collection tuning (async runtime specific)

### Concurrency & Async Runtime
- Thread pool sizing (test with high concurrency)
- Async runtime configuration (compare sync vs async libraries)
- Lock contention reduction (test under high load)

### Network & Protocol
- TCP socket options (test with various network conditions)
- HTTP/2 vs HTTP/1.1 settings (comparative benchmarking)
- Connection warmup strategies (test cold start performance)

## Best Practices

1. **One Factor Per Iteration**: Change only one performance parameter at a time
2. **Statistical Rigor**: Run at least 3 benchmarks and calculate averages with standard deviation
3. **Strategy-Specific Testing**: Adjust concurrency/request parameters based on optimization target
4. **Library Selection**: Choose sync vs async libraries based on optimization focus
5. **Consistent Testing**: Use identical environmental conditions for comparisons
6. **Clear Documentation**: Record reasoning and expected outcomes in `tests_refactoring/` files
7. **Rollback Ready**: Keep branches until optimization is proven successful over multiple runs
8. **Metric Focused**: Define specific, measurable improvement targets before implementation

## Success Criteria

An optimization is considered successful if it achieves:
- **≥5% improvement** in target metric (average across 3+ runs)
- **No significant regression** in other metrics (≤2% degradation acceptable)
- **Stable performance** across multiple benchmark runs (low standard deviation)
- **Statistical significance** with consistent results
- **Code maintainability** preserved or improved
- **Documentation completeness** in `tests_refactoring/optimization-{strategy-name}.md`

### Measurement Standards
- **Minimum runs**: 3 benchmarks per configuration
- **Acceptable variation**: Standard deviation ≤10% of mean
- **Significance threshold**: Improvement must exceed measurement noise
- **Regression tolerance**: Other metrics may degrade by ≤2%