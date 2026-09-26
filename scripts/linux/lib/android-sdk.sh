#!/usr/bin/env bash
# Pinned Android SDK platform/build-tools bootstrap shared by Linux Android builds.

find_android_platform_jar() {
    local sdk_root="$1"
    local api_level="$2"
    python3 - "$sdk_root" "$api_level" <<'PYPLATFORM'
from pathlib import Path
import re, sys
root = Path(sys.argv[1]) / "platforms"
required = int(sys.argv[2])
for candidate in sorted(root.glob("android-*")) if root.is_dir() else []:
    jar = candidate / "android.jar"
    if not jar.is_file():
        continue
    api = None
    props = candidate / "source.properties"
    if props.is_file():
        text = props.read_text(errors="replace")
        match = re.search(r"(?m)^AndroidVersion\.ApiLevel\s*=\s*(\d+)\s*$", text)
        if match:
            api = int(match.group(1))
    if api is None:
        match = re.fullmatch(r"android-(\d+)(?:\.\d+)?", candidate.name)
        if match:
            api = int(match.group(1))
    if api == required:
        print(jar)
        raise SystemExit(0)
raise SystemExit(1)
PYPLATFORM
}

ensure_android_command_line_tools() {
    local sdk_root="$1"
    local toolchains_env="$2"
    local sdkmanager="$sdk_root/cmdline-tools/latest/bin/sdkmanager"
    if [[ -x "$sdkmanager" ]]; then
        printf '%s\n' "$sdkmanager"
        return 0
    fi
    local revision="${KASKOLD_ANDROID_CMDLINE_TOOLS:-}"
    local expected="${KASKOLD_ANDROID_CMDLINE_TOOLS_LINUX_SHA256:-}"
    [[ -n "$revision" && "$expected" =~ ^[0-9A-Fa-f]{64}$ ]] \
        || fail "Pinned Android command-line tools metadata is missing or invalid in $toolchains_env."
    info "Android command-line tools $revision are not installed; downloading and verifying them under $sdk_root" >&2
    python3 - "$sdk_root" "$revision" "$expected" <<'PYSDK'
from hashlib import sha256
from pathlib import Path
import shutil, sys, tempfile, urllib.request, zipfile
sdk = Path(sys.argv[1])
revision = sys.argv[2]
expected = sys.argv[3].lower()
target = sdk / "cmdline-tools" / "latest"
url = f"https://dl.google.com/android/repository/commandlinetools-linux-{revision}_latest.zip"
with tempfile.TemporaryDirectory(prefix="kaskold-android-tools-") as temp_name:
    temp = Path(temp_name)
    archive = temp / "cmdline-tools.zip"
    with urllib.request.urlopen(url) as response, archive.open("wb") as output:
        shutil.copyfileobj(response, output)
    h = sha256()
    with archive.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            h.update(chunk)
    actual = h.hexdigest()
    if actual != expected:
        raise SystemExit(f"ERROR: Android command-line tools SHA-256 mismatch: expected {expected}, got {actual}")
    unpack = temp / "unpack"
    with zipfile.ZipFile(archive) as package:
        package.extractall(unpack)
    source = unpack / "cmdline-tools"
    if not (source / "bin" / "sdkmanager").is_file():
        raise SystemExit("ERROR: Android command-line tools archive is missing sdkmanager.")
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.rmtree(target, ignore_errors=True)
    shutil.move(str(source), str(target))
for path in (target / "bin").iterdir():
    if path.is_file():
        path.chmod(path.stat().st_mode | 0o100)
PYSDK
    [[ -x "$sdkmanager" ]] || fail "Pinned Android command-line tools could not be prepared under $sdk_root."
    printf '%s\n' "$sdkmanager"
}

install_android_sdk_packages() {
    local sdk_root="$1"
    local toolchains_env="$2"
    shift 2
    local packages=("$@")
    (( ${#packages[@]} )) || return 0

    local sdkmanager android_cli tools_bin accept_input
    sdkmanager="$(ensure_android_command_line_tools "$sdk_root" "$toolchains_env")"
    tools_bin="$sdk_root/cmdline-tools/latest/bin"
    android_cli="$tools_bin/android"
    accept_input="$(printf 'y\n%.0s' {1..100})"

    if [[ -x "$android_cli" ]]; then
        if "$android_cli" --sdk="$sdk_root" sdk install --canary "${packages[@]}" <<<"$accept_input"; then
            return 0
        fi
        printf 'WARNING: Android CLI package installation failed; retrying with sdkmanager preview-channel compatibility mode.\n' >&2
    fi

    "$sdkmanager" --sdk_root="$sdk_root" --licenses <<<"$accept_input" >/dev/null \
        || fail "Android SDK license acceptance failed."
    local legacy=() package
    for package in "${packages[@]}"; do
        legacy+=("${package/\//;}")
    done
    "$sdkmanager" --sdk_root="$sdk_root" --channel=3 "${legacy[@]}" \
        || return 1
}

ensure_android_sdk_build_packages() {
    local sdk_root="$1"
    local toolchains_env="$2"
    local api="${KASKOLD_ANDROID_API:-}"
    local build_tools="${KASKOLD_ANDROID_BUILD_TOOLS:-}"
    [[ "$api" =~ ^[0-9]+$ ]] || fail "KASKOLD_ANDROID_API is missing or invalid in $toolchains_env."
    [[ -n "$build_tools" ]] || fail "KASKOLD_ANDROID_BUILD_TOOLS is missing from $toolchains_env."
    local platform_jar="" build_tools_bin="$sdk_root/build-tools/$build_tools/aapt2"
    platform_jar="$(find_android_platform_jar "$sdk_root" "$api" 2>/dev/null || true)"
    if [[ -n "$platform_jar" && -x "$build_tools_bin" ]]; then
        printf '%s\n' "$platform_jar"
        return 0
    fi

    info "Preparing pinned Android SDK API $api / build-tools $build_tools under $sdk_root" >&2
    local candidate packages=()
    for candidate in "platforms/android-$api" "platforms/android-$api.0"; do
        packages=()
        [[ -n "$platform_jar" ]] || packages+=("$candidate")
        [[ -x "$build_tools_bin" ]] || packages+=("build-tools/$build_tools")
        if install_android_sdk_packages "$sdk_root" "$toolchains_env" "${packages[@]}"; then
            platform_jar="$(find_android_platform_jar "$sdk_root" "$api" 2>/dev/null || true)"
            if [[ -n "$platform_jar" && -x "$build_tools_bin" ]]; then
                printf '%s\n' "$platform_jar"
                return 0
            fi
        fi
    done

    [[ -n "$platform_jar" ]] || fail "Android SDK platform API $api could not be prepared under $sdk_root/platforms."
    fail "Android build-tools $build_tools could not be prepared under $sdk_root/build-tools."
}

