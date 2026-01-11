#!/usr/bin/env python3
"""Run comprehensive POST payload benchmarks using http-benchmark CLI."""

import subprocess
import json

# Payload sizes and files
payloads = [
    ("50B", "tests/test_post/payload_50b.json"),
    ("1KB", "tests/test_post/payload_1kb.json"),
    ("10KB", "tests/test_post/payload_10kb.json"),
    ("100KB", "tests/test_post/payload_100kb.json"),
    ("512KB", "tests/test_post/payload_512kb.json"),
    ("1MB", "tests/test_post/payload_1mb.json"),
    ("2MB", "tests/test_post/payload_2mb.json"),
]

results = []

print("=" * 80)
print("RequestX POST Benchmark - Payload Size Tests")
print("=" * 80)
print("Configuration: POST /localhost/post | 1 concurrency | 3 seconds")
print("=" * 80)

for name, filepath in payloads:
    # Read payload
    with open(filepath, 'r') as f:
        payload = f.read()
    payload_size = len(payload)
    
    print(f"\n{'='*80}")
    print(f"Payload: {name} ({payload_size:,} bytes)")
    print(f"{'='*80}")
    
    # Run requestx benchmark using uv run
    cmd1 = [
        "uv", "run", "http-benchmark",
        "--url", "http://localhost/post",
        "--method", "POST",
        "--body", payload,
        "--headers", '{"Content-Type": "application/json"}',
        "--client", "requestx",
        "--concurrency", "1",
        "--duration", "3"
    ]
    
    result1 = subprocess.run(cmd1, capture_output=True, text=True, cwd="/Users/qunfei.wu/Projects/requestx")
    
    # Parse requestx results
    rps1 = cpu1 = mem1 = err1 = 0.0
    for line in result1.stdout.split('\n'):
        line = line.strip()
        if 'RPS:' in line:
            try:
                rps1 = float(line.split('RPS:')[1].split()[0].replace(',', ''))
            except:
                rps1 = 0.0
        elif 'CPU Usage (avg):' in line:
            try:
                cpu1 = float(line.split('CPU Usage (avg):')[1].split()[0])
            except:
                cpu1 = 0.0
        elif 'Memory Usage (avg):' in line:
            try:
                mem1 = float(line.split('Memory Usage (avg):')[1].split()[0])
            except:
                mem1 = 0.0
        elif 'Error Rate:' in line:
            try:
                err1 = float(line.split('Error Rate:')[1].split()[0])
            except:
                err1 = 0.0
    
    # Run requests benchmark using uv run
    cmd2 = [
        "uv", "run", "http-benchmark",
        "--url", "http://localhost/post",
        "--method", "POST",
        "--body", payload,
        "--headers", '{"Content-Type": "application/json"}',
        "--client", "requests",
        "--concurrency", "1",
        "--duration", "3"
    ]
    
    result2 = subprocess.run(cmd2, capture_output=True, text=True, cwd="/Users/qunfei.wu/Projects/requestx")
    
    # Parse requests results
    rps2 = cpu2 = mem2 = err2 = 0.0
    for line in result2.stdout.split('\n'):
        line = line.strip()
        if 'RPS:' in line:
            try:
                rps2 = float(line.split('RPS:')[1].split()[0].replace(',', ''))
            except:
                rps2 = 0.0
        elif 'CPU Usage (avg):' in line:
            try:
                cpu2 = float(line.split('CPU Usage (avg):')[1].split()[0])
            except:
                cpu2 = 0.0
        elif 'Memory Usage (avg):' in line:
            try:
                mem2 = float(line.split('Memory Usage (avg):')[1].split()[0])
            except:
                mem2 = 0.0
        elif 'Error Rate:' in line:
            try:
                err2 = float(line.split('Error Rate:')[1].split()[0])
            except:
                err2 = 0.0
    
    # Calculate improvements
    speedup = ((rps1 - rps2) / rps2) * 100 if rps2 > 0 else 0
    cpu_save = ((cpu2 - cpu1) / cpu2) * 100 if cpu2 > 0 else 0
    
    print(f"┌{'─'*40}┬{'─'*20}┬{'─'*20}┐")
    print(f"│ {'Metric':^38} │ {'requestx':^18} │ {'requests':^18} │")
    print(f"├{'─'*40}┼{'─'*20}┼{'─'*20}┤")
    print(f"│ {'RPS':^38} │ {rps1:>18,.2f} │ {rps2:>18,.2f} │")
    print(f"│ {'CPU Usage (%)':^38} │ {cpu1:>18.2f} │ {cpu2:>18.2f} │")
    print(f"│ {'Memory (MB)':^38} │ {mem1:>18.2f} │ {mem2:>18.2f} │")
    print(f"│ {'Error Rate (%)':^38} │ {err1:>18.2f} │ {err2:>18.2f} │")
    print(f"└{'─'*40}┴{'─'*20}┴{'─'*20}┘")
    
    print(f"\n  📈 Speedup: +{speedup:.1f}% | CPU Savings: {cpu_save:.1f}%")
    
    results.append({
        "name": name,
        "size": payload_size,
        "rps_requestx": rps1,
        "rps_requests": rps2,
        "speedup": speedup,
        "cpu_requestx": cpu1,
        "cpu_requests": cpu2,
        "cpu_save": cpu_save,
        "mem_requestx": mem1,
        "mem_requests": mem2,
        "err_requestx": err1,
        "err_requests": err2
    })

print("\n" + "=" * 80)
print("SUMMARY TABLE")
print("=" * 80)
print(f"{'Payload':<10} {'Size':>10} {'RequestX RPS':>14} {'requests RPS':>14} {'Speedup':>10} {'CPU Save':>10}")
print("-" * 80)
for r in results:
    print(f"{r['name']:<10} {r['size']:>10,} {r['rps_requestx']:>14,.2f} {r['rps_requests']:>14,.2f} {r['speedup']:>9.1f}% {r['cpu_save']:>9.1f}%")
print("=" * 80)

# Save results to JSON for chart generation
with open("/tmp/benchmark_results.json", "w") as f:
    json.dump(results, f, indent=2)
print("\n✓ Results saved to /tmp/benchmark_results.json")
