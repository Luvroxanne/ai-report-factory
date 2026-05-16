from __future__ import annotations

import json
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
VERSION_RE = re.compile(r"^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$")


def write_json(path: Path, data: dict) -> None:
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def replace_version_line(path: Path, pattern: str, replacement: str) -> None:
    text = path.read_text(encoding="utf-8")
    new_text, count = re.subn(pattern, replacement, text, count=1, flags=re.MULTILINE)
    if count != 1:
        raise RuntimeError(f"未找到版本号位置：{path}")
    path.write_text(new_text, encoding="utf-8")


def main() -> int:
    if len(sys.argv) != 2:
        print("用法：python scripts/bump_version.py 0.2.1")
        return 2

    version = sys.argv[1].lstrip("v").strip()
    if not VERSION_RE.match(version):
        print("版本号格式错误，例如：0.2.1 或 1.0.0")
        return 2

    tauri_conf = ROOT / "src-tauri" / "tauri.conf.json"
    data = json.loads(tauri_conf.read_text(encoding="utf-8"))
    data["version"] = version
    write_json(tauri_conf, data)

    replace_version_line(
        ROOT / "src-tauri" / "Cargo.toml",
        r'^version\s*=\s*".*"$',
        f'version = "{version}"',
    )

    package_json = ROOT / "frontend" / "package.json"
    data = json.loads(package_json.read_text(encoding="utf-8"))
    data["version"] = version
    write_json(package_json, data)

    package_lock = ROOT / "frontend" / "package-lock.json"
    if package_lock.exists():
        data = json.loads(package_lock.read_text(encoding="utf-8"))
        data["version"] = version
        if "" in data.get("packages", {}):
            data["packages"][""]["version"] = version
        write_json(package_lock, data)

    replace_version_line(
        ROOT / "backend" / "app" / "version.py",
        r'^APP_VERSION\s*=\s*".*"$',
        f'APP_VERSION = "{version}"',
    )

    print(f"版本号已同步为 v{version}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
