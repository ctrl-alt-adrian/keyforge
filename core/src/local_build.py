"""Executed only inside the pinned official ZMK image; no host Python required."""
import hashlib
import itertools
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import subprocess
import sys

import yaml  # Included with West/Zephyr in the official image.


def run(*args, capture=False):
    print("+ " + shlex.join(args), flush=True)
    return subprocess.run(args, check=True, text=True, stdout=subprocess.PIPE if capture else None)


def words(value):
    if value is None:
        return []
    if isinstance(value, str):
        return shlex.split(value)
    if isinstance(value, list) and all(isinstance(v, str) for v in value):
        return value
    raise ValueError("cmake-args must be a string or list of strings")


def matrix(data):
    if not isinstance(data, dict):
        raise ValueError("build.yaml must contain a build matrix")
    axes = {k: v for k, v in data.items() if k not in ("include", "exclude")}
    if any(not isinstance(v, list) or not v for v in axes.values()):
        raise ValueError("Build matrix axes must be nonempty arrays")
    base = [dict(zip(axes, values)) for values in itertools.product(*axes.values())] if axes else []
    excluded = data.get("exclude", [])
    included = data.get("include", [])
    if not isinstance(excluded, list) or not isinstance(included, list) or any(not isinstance(v, dict) for v in excluded + included):
        raise ValueError("include/exclude must be arrays of matrix entries")
    base = [row for row in base if not any(all(row.get(k) == v for k, v in ex.items()) for ex in excluded)]
    result = [row.copy() for row in base]
    for extra in included:
        matches = [i for i, original in enumerate(base) if all(k not in original or original[k] == v for k, v in extra.items())]
        if matches:
            for i in matches:
                result[i].update(extra)
        else:
            result.append(extra.copy())
    if not result or len(result) > 256:
        raise ValueError("build.yaml must resolve to between 1 and 256 targets")
    return result


def main():
    source = Path("/source")
    work = Path("/work")
    Path(os.environ["HOME"]).mkdir(exist_ok=True)
    os.environ.setdefault("XDG_CACHE_HOME", "/work/home/.cache")
    sdks = sorted(Path("/opt").glob("zephyr-sdk-*"))
    if sdks:
        os.environ.setdefault("ZEPHYR_SDK_INSTALL_DIR", str(sdks[-1]))
    config = source / "config" if (source / "config/west.yml").is_file() else source
    targets = matrix(yaml.safe_load((source / "build.yaml").read_text()))
    names = []
    for i, target in enumerate(targets):
        board = target.get("board")
        if not isinstance(board, str) or not board or board.startswith("-"):
            raise ValueError(f"Target {i + 1} needs a board")
        name = target.get("artifact-name") or f"{target.get('shield', '')}-{board}-zmk"
        name = re.sub(r"[^a-zA-Z0-9_.-]", "_", name).strip(".")
        if not name or name in names:
            raise ValueError("Each target needs a distinct artifact-name")
        names.append(name)
    snapshot = work / "config"
    if snapshot.exists():
        shutil.rmtree(snapshot)
    shutil.copytree(config, snapshot, ignore=shutil.ignore_patterns(".git", ".west", ".keyforge", "node_modules", "target", "build"))
    digest = hashlib.sha256()
    for file in sorted(snapshot.rglob("*")):
        if file.is_file() and file.suffix in (".yml", ".yaml"):
            digest.update(str(file.relative_to(snapshot)).encode())
            digest.update(file.read_bytes())
    stamp = work / "manifest.sha256"
    changed = not stamp.exists() or stamp.read_text() != digest.hexdigest()
    if not (work / ".west").exists():
        run("west", "init", "-l", str(snapshot))
        changed = True
    if changed:
        run("west", "update", "--fetch-opt=--filter=tree:0")
        run("west", "zephyr-export")
        frozen = run("west", "manifest", "--freeze", "--active-only", capture=True).stdout
        (work / "west-frozen.yml").write_text(frozen)
        stamp.write_text(digest.hexdigest())
    else:
        print("Reusing cached West sources; manifest unchanged.", flush=True)
    app = work / "zmk/app"
    if not app.is_dir():
        raise ValueError("West manifest must provide ZMK at zmk/app")
    artifacts = work / "artifacts" / sys.argv[1]
    artifacts.mkdir(parents=True)
    failures = []
    jobs = max(1, int(os.environ["KEYFORGE_BUILD_JOBS"]))
    for index, (target, name) in enumerate(zip(targets, names)):
        build = work / "build" / name
        args = ["west", "build", "-p", "auto", "-s", str(app), "-d", str(build), "-b", target["board"], f"-o=-j{jobs}"]
        if target.get("snippet"):
            args += ["-S", str(target["snippet"])]
        args += ["--", f"-DZMK_CONFIG={config}"]
        if target.get("shield"):
            args.append(f"-DSHIELD={target['shield']}")
        if (source / "zephyr/module.yml").is_file():
            args.append("-DZMK_EXTRA_MODULES=/source")
        args += words(target.get("cmake-args"))
        print(f"Building target {index + 1}/{len(targets)}: {name}", flush=True)
        try:
            run(*args)
            firmware = next((build / "zephyr" / f"zmk.{ext}" for ext in ("uf2", "bin", "hex") if (build / "zephyr" / f"zmk.{ext}").is_file()), None)
            if firmware is None:
                raise ValueError(f"No firmware output for {name}")
            shutil.copy2(firmware, artifacts / (name + firmware.suffix))
        except (subprocess.CalledProcessError, ValueError) as error:
            failures.append(name)
            print(f"Target failed: {name}: {error}", flush=True)
    (work / "last-build.json").write_text(json.dumps({"image": os.environ.get("ZEPHYR_VERSION"), "targets": targets, "failed": failures}, indent=2))
    if failures:
        raise ValueError("Failed targets: " + ", ".join(failures))


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, subprocess.CalledProcessError, yaml.YAMLError) as error:
        print(f"Local build error: {error}", file=sys.stderr, flush=True)
        sys.exit(1)
