"""Neutral, bounded CI wiring fixture; no dependency, service or model access."""

import hashlib
import json
from pathlib import Path
import sys
import platform


def normalize(value):
    return " ".join(value.split())


def main():
    mode = sys.argv[1]
    source = Path(__file__)
    output = Path("feedback-artifacts")
    if mode == "diagnose":
        observed = {
            "kind": "diagnostic",
            "qualification": "unverified",
            "system": platform.system(),
            "python": platform.python_version(),
            "whitespace_result": normalize(" alpha\t beta "),
        }
        print(json.dumps(observed))
        if len(sys.argv) != 3 or observed["system"] != sys.argv[2]:
            raise RuntimeError("diagnostic expectation differs from observed environment")
    elif mode == "verify":
        assert normalize("  alpha\t beta\n") == "alpha beta"
        assert normalize("") == ""
        output.mkdir(exist_ok=True)
        report = output / "verification.json"
        if report.exists():
            raise RuntimeError("verification already ran in this workspace")
        report.write_text(
            json.dumps({"source_sha256": hashlib.sha256(source.read_bytes()).hexdigest()})
        )
    elif mode == "candidate":
        expected = json.loads((output / "verification.json").read_text())["source_sha256"]
        data = source.read_bytes()
        if hashlib.sha256(data).hexdigest() != expected:
            raise RuntimeError("source changed after verification")
        with (output / "candidate.py").open("xb") as candidate:
            candidate.write(data)
    elif mode == "consume":
        expected = json.loads((output / "verification.json").read_text())["source_sha256"]
        if hashlib.sha256((output / "candidate.py").read_bytes()).hexdigest() != expected:
            raise RuntimeError("candidate differs from verified source")
    else:
        raise ValueError("unknown fixture action")


if __name__ == "__main__":
    main()
