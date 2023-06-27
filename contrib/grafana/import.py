#!/bin/env python3

import argparse
from collections import namedtuple
from enum import Enum
import json
import logging
import os
from pathlib import Path
import string
import subprocess
import random
import tempfile
import time

DOCKER_BIN = os.environ.get("DOCKER", "docker")
METRICS = [
    "cgroup",
    "disk",
    "iface",
    "network",
    "process",
    "system",
    "transport",
]


def get_below_bin(snapshot):
    """Get the below "binary" to run"""
    env = os.environ.get("BELOW")
    if env:
        return env.split()
    else:
        # Always use the latest docker image
        subprocess.run(["docker", "pull", "below/below:latest"], check=True)

        volume_args = []
        if snapshot:
            volume_args += ["-v", f"{snapshot}:{snapshot}"]
        else:
            # Else we are importing from localhost
            store_dir = "/var/log/below/"
            volume_args += ["-v", f"{store_dir}:{store_dir}"]

        return [
            "docker",
            "run",
            "--rm",
            *volume_args,
            "below/below:latest",
        ]


def dump(source, category, begin, end, outfile):
    """Gets below to dump openmetrics data to given outfile"""
    if source.lower() == "host":
        below_source = None
        below_source_args = []
    else:
        below_source = str(Path(source).absolute())
        below_source_args = ["--snapshot", below_source]

    cmd = [
        *get_below_bin(below_source),
        "dump",
        *below_source_args,
        category,
        "--begin",
        begin,
        "--end",
        end,
        "--everything",
        "--output-format",
        "openmetrics",
    ]
    cmd_str = " ".join(cmd)
    logging.info(f"Dumping {category} data with cmd='{cmd_str}'")

    process = subprocess.run(
        cmd, stdout=outfile, stderr=subprocess.PIPE, encoding="utf-8"
    )
    if process.returncode != 0:
        logging.error(f"process stderr={process.stderr}")
        raise RuntimeError(f"Failed to dump {category} data: {process.stderr}")


def ingest(metrics_file):
    """Ingest metrics into prometheus"""
    subprocess.run(
        [DOCKER_BIN, "compose", "cp", metrics_file, "prometheus:/import.txt"],
        check=True,
    )
    subprocess.run(
        [
            DOCKER_BIN,
            "compose",
            "exec",
            "prometheus",
            "promtool",
            "tsdb",
            "create-blocks-from",
            "openmetrics",
            "/import.txt",
            "/prometheus",
        ],
        check=True,
    )


def restart_prometheus():
    subprocess.run([DOCKER_BIN, "compose", "restart", "prometheus"], check=True)


def do_import(begin, end, source):
    logging.info(f"Importing {source} from '{begin}' to '{end}'")
    for category in METRICS:
        with tempfile.NamedTemporaryFile(mode="w") as f:
            data = dump(source, category, begin, end, f)
            # Need to chmod b/c there's a uid/gid mismatch when copying into container
            os.chmod(f.name, 0o644)
            ingest(f.name)
    restart_prometheus()


def main():
    parser = argparse.ArgumentParser(description="Imports below data into prometheus")
    parser.add_argument("--begin", "-b", default="99 years ago", help="Import start")
    parser.add_argument("--end", "-e", default="now", help="Import end")
    parser.add_argument("source", help="Path to snapshot or `host`, for local host")
    args = parser.parse_args()

    start = time.time()
    do_import(args.begin, args.end, args.source)
    logging.info(f"Done in {time.time() - start}s")


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)
    main()
