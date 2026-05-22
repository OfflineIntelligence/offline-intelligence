"""
llama-bench — Offline Intelligence SDK Raw Throughput Benchmark
================================================================

Measures token generation (tg) and prompt processing (pp) speed directly
via llama-bench.exe from the bundled llama.cpp build. Zero HTTP/SDK overhead
— this is the pure engine number that all latency is built on top of.

Two configurations are run back-to-back:

  Config A  OI-Optimized
            All flags OI uses in production:
            --flash-attn 1 --cache-type-k q8_0 --cache-type-v q8_0
            --ubatch-size 1024 --threads 6
            This is what OI delivers to every user.

  Config B  Bare-minimum (wrapper baseline)
            --n-gpu-layers 28 only.
            This simulates the default config shipped by Ollama, LM Studio,
            and Jan.ai when you install them and load the same model.

Usage:
    python benchmarks/llama_bench.py
    python benchmarks/llama_bench.py --llama-bench /path/to/llama-bench.exe
    python benchmarks/llama_bench.py --model /path/to/model.gguf
    python benchmarks/llama_bench.py --reps 3   # fewer reps for quick test
"""

import argparse
import json
import os
import platform
import subprocess
import sys
from datetime import datetime, timezone

# ---------------------------------------------------------------------------
# Defaults (match your local setup)
# ---------------------------------------------------------------------------

LLAMA_BENCH_DEFAULT = (
    r"C:/Users/pamar/AppData/Roaming/OfflineIntelligence/engines/"
    r"llama-cuda-windows-x64-b8037/llama-bench.exe"
)
MODEL_DEFAULT = (
    r"C:/Users/pamar/Downloads/qwen2.5-coder-3b-instruct-q4_k_m.gguf"
)
RESULTS_DIR = os.path.join(os.path.dirname(__file__), "results")


# ---------------------------------------------------------------------------
# Configuration definitions
# ---------------------------------------------------------------------------

def build_configs(model: str, reps: int) -> list[dict]:
    """Return the two benchmark configurations."""
    base_flags = [
        "--model", model,
        "--n-gpu-layers", "28",
        "-r", str(reps),
        "--output", "json",
    ]

    oi_flags = [
        "--flash-attn", "1",
        "--cache-type-k", "q8_0",
        "--cache-type-v", "q8_0",
        "--ubatch-size", "1024",
        "--threads", "6",
    ]

    # pp-only: vary prompt length, generate 0 tokens
    # tg-only: 0-token prompt, vary generation length
    test_suite = ["-p", "128,512,1024", "-n", "0,128,256,512"]

    return [
        {
            "name": "OI-Optimized",
            "label": "Offline Intelligence SDK — Optimized flags",
            "flags": base_flags + oi_flags + test_suite,
            "description": (
                "--flash-attn 1  --cache-type-k q8_0  --cache-type-v q8_0  "
                "--ubatch-size 1024  --threads 6"
            ),
        },
        {
            "name": "Bare-GPU-only",
            "label": "Bare llama.cpp — GPU layers only (wrapper baseline)",
            "flags": base_flags + test_suite,
            "description": "--n-gpu-layers 28  (no other optimizations)",
        },
    ]


# ---------------------------------------------------------------------------
# Run llama-bench
# ---------------------------------------------------------------------------

def run_config(llama_bench: str, cfg: dict) -> list[dict]:
    """Run llama-bench for one config. Returns raw JSON rows."""
    cmd = [llama_bench] + cfg["flags"]
    print(f"\n  Running: {' '.join(cmd[:6])} ... ({len(cmd)} args total)")

    result = subprocess.run(
        cmd,
        capture_output=True,
        text=True,
    )

    if result.returncode != 0:
        print(f"  ERROR (exit {result.returncode}):")
        print(result.stderr[-800:] if result.stderr else "(no stderr)")
        return []

    try:
        rows = json.loads(result.stdout)
        if not isinstance(rows, list):
            rows = [rows]
        return rows
    except json.JSONDecodeError as exc:
        print(f"  JSON parse error: {exc}")
        print("  stdout:", result.stdout[:500])
        return []


# ---------------------------------------------------------------------------
# Summarise results
# ---------------------------------------------------------------------------

def summarise(rows: list[dict]) -> dict:
    """
    Split rows into pp (prompt processing) and tg (token generation) groups.

    llama-bench row discrimination:
      - pp row: n_gen == 0  (only processes the prompt, generates nothing)
      - tg row: n_prompt == 0  (no prompt, pure generation from BOS)
    """
    pp_rows = [r for r in rows if r.get("n_gen", -1) == 0]
    tg_rows = [r for r in rows if r.get("n_prompt", -1) == 0]

    def stats(subset: list[dict]) -> dict | None:
        speeds = [r["avg_ts"] for r in subset if "avg_ts" in r]
        if not speeds:
            return None
        return {
            "mean_ts":  round(sum(speeds) / len(speeds), 2),
            "min_ts":   round(min(speeds), 2),
            "max_ts":   round(max(speeds), 2),
            "n_samples": len(speeds),
            "per_test": [
                {
                    "n_prompt": r.get("n_prompt", 0),
                    "n_gen":    r.get("n_gen", 0),
                    "avg_ts":   round(r["avg_ts"], 2),
                    "std_ts":   round(r.get("std_ts", 0.0), 2),
                }
                for r in subset
            ],
        }

    return {
        "prompt_processing": stats(pp_rows),
        "token_generation":  stats(tg_rows),
    }


# ---------------------------------------------------------------------------
# Hardware info
# ---------------------------------------------------------------------------

def hardware_info() -> dict:
    info = {
        "platform": platform.platform(),
        "python":   sys.version.split()[0],
    }
    try:
        import subprocess as sp
        gpu = sp.run(
            ["nvidia-smi", "--query-gpu=name,memory.total,driver_version",
             "--format=csv,noheader"],
            capture_output=True, text=True
        )
        if gpu.returncode == 0:
            info["gpu"] = gpu.stdout.strip()
    except Exception:
        pass
    return info


# ---------------------------------------------------------------------------
# Pretty-print
# ---------------------------------------------------------------------------

def print_summary(configs: list[dict], summaries: list[dict]) -> None:
    sep = "=" * 68
    print(f"\n{sep}")
    print("RESULTS")
    print(sep)

    for cfg, s in zip(configs, summaries):
        print(f"\n  {cfg['label']}")
        print(f"  Flags: {cfg['description']}")
        print()

        tg = s.get("token_generation")
        pp = s.get("prompt_processing")

        if tg:
            print(f"  Token generation (T/s)  -- what users feel as 'response speed'")
            for pt in tg["per_test"]:
                print(f"    gen={pt['n_gen']:>4} tok : {pt['avg_ts']:>7.2f} T/s  (+/-{pt['std_ts']:.2f})")
            print(f"    MEAN across all gen lengths : {tg['mean_ts']:.2f} T/s")
        else:
            print("  Token generation: no data")

        print()

        if pp:
            print(f"  Prompt processing (T/s)  -- how fast context is ingested")
            for pt in pp["per_test"]:
                print(f"    ctx={pt['n_prompt']:>5} tok : {pt['avg_ts']:>9.2f} T/s")
            print(f"    MEAN across all ctx lengths : {pp['mean_ts']:.2f} T/s")
        else:
            print("  Prompt processing: no data")

        print()

    print(sep)


# ---------------------------------------------------------------------------
# Save results
# ---------------------------------------------------------------------------

def save_results(
    configs: list[dict],
    summaries: list[dict],
    raw_rows: list[list[dict]],
    model: str,
    reps: int,
) -> str:
    os.makedirs(RESULTS_DIR, exist_ok=True)
    ts = datetime.now(timezone.utc).strftime("%Y%m%d_%H%M%S")
    out_path = os.path.join(RESULTS_DIR, f"llama_bench_{ts}.json")

    payload = {
        "benchmark": "llama-bench",
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "model": model,
        "repetitions": reps,
        "hardware": hardware_info(),
        "engine": "llama.cpp b8037 CUDA 12.4",
        "configurations": [
            {
                "name":        cfg["name"],
                "label":       cfg["label"],
                "description": cfg["description"],
                "summary":     s,
                "raw_rows":    rows,
            }
            for cfg, s, rows in zip(configs, summaries, raw_rows)
        ],
    }

    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(payload, f, indent=2)

    return out_path


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(description="OI llama-bench wrapper")
    parser.add_argument("--llama-bench", default=LLAMA_BENCH_DEFAULT,
                        help="Path to llama-bench.exe")
    parser.add_argument("--model", default=MODEL_DEFAULT,
                        help="Path to GGUF model file")
    parser.add_argument("--reps", type=int, default=5,
                        help="Repetitions per test (default 5)")
    args = parser.parse_args()

    if not os.path.exists(args.llama_bench):
        sys.exit(f"llama-bench not found: {args.llama_bench}")
    if not os.path.exists(args.model):
        sys.exit(f"Model not found: {args.model}")

    sep = "=" * 68
    print(sep)
    print("Offline Intelligence SDK — llama-bench")
    print(sep)
    print(f"  llama-bench : {args.llama_bench}")
    print(f"  Model       : {args.model}")
    print(f"  Repetitions : {args.reps}")
    print(f"  GPU         : RTX 3050 Ti 4 GB  (CUDA 12.4)")
    hw = hardware_info()
    if "gpu" in hw:
        print(f"  Detected GPU: {hw['gpu']}")
    print(f"\nRunning 2 configurations x {args.reps} reps each.")
    print("This will take approximately 6-10 minutes. Please wait...\n")

    configs = build_configs(args.model, args.reps)
    all_rows: list[list[dict]] = []
    all_summaries: list[dict] = []

    for i, cfg in enumerate(configs, 1):
        print(f"[{i}/{len(configs)}] {cfg['label']}")
        rows = run_config(args.llama_bench, cfg)
        s = summarise(rows)
        all_rows.append(rows)
        all_summaries.append(s)

    print_summary(configs, all_summaries)

    out_path = save_results(configs, all_summaries, all_rows, args.model, args.reps)
    print(f"Results saved: {out_path}\n")


if __name__ == "__main__":
    main()
