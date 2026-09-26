# KasKold — reproducible v2.0.0 release build
#
# The supported reproducible-build path is Docker-only. The convenience runner:
#   ./scripts/linux/build/reproducible-build.sh
# provisions the pinned toolchain image, performs this build, and exports the
# artifacts. No release build step flashes or contacts a hardware device.
#
# Manual toolchain image build:
#   docker build --platform linux/amd64 -f Dockerfile.base -t kaskold-toolchain:v3 .
# Manual verifier export (unsigned images):
#   DOCKER_BUILDKIT=1 docker build --platform linux/amd64 --target artifacts \
#     --output type=local,dest=release .
# Manual maintainer export (signed + unsigned images):
#   DOCKER_BUILDKIT=1 docker build --platform linux/amd64 --target artifacts \
#     --secret id=signkey,src=/path/to/dev_signing_key.bin \
#     --output type=local,dest=release .
#
# The qualified CoreS3 release target produces four firmware images:
# signed/unsigned x app-only/full-flash. CoreS3 Lite remains build-supported
# but is intentionally excluded from production artifacts until HIL-qualified.
FROM --platform=linux/amd64 kaskold-toolchain:v3 AS builder

SHELL ["/bin/bash", "-c"]
WORKDIR /build/KasKold

COPY Cargo.toml Cargo.lock ./
COPY apps/ apps/
COPY crates/ crates/
COPY external/ external/
COPY qa/ qa/
COPY tools/ tools/
ARG KASKOLD_GIT_COMMIT=
ENV KASKOLD_GIT_COMMIT=${KASKOLD_GIT_COMMIT}
ENV SOURCE_DATE_EPOCH=0
ENV TZ=UTC
ENV LC_ALL=C
ENV LANG=C
ENV CARGO_INCREMENTAL=0

# Capture immutable source inputs before five-pass convergence intentionally
# rewrites apps/kaskold-hardware/src/firmware_hash.rs.
RUN find Cargo.toml Cargo.lock apps crates external qa tools \
        -type f \
        ! -path '*/target/*' \
        ! -path 'apps/kaskold-hardware/src/firmware_hash.rs' \
        -print0 \
    | sort -z \
    | xargs -0 sha256sum \
    > /build/SOURCE-SHA256SUMS

# Verify the browser companion using the pinned host Rust toolchain. Firmware
# release artifacts are produced below with the pinned ESP toolchain.
RUN source /etc/kaskold/toolchains.env && \
    rustup target list --toolchain "$KASKOLD_REPRO_HOST_RUST" --installed | grep -qx wasm32-unknown-unknown && \
    cd apps/kaskold-companion-web && \
    cargo "+$KASKOLD_REPRO_HOST_RUST" build --offline --locked --target wasm32-unknown-unknown --release

RUN source /etc/kaskold/toolchains.env && \
    rustup target list --toolchain "$KASKOLD_REPRO_HOST_RUST" --installed | grep -qx wasm32-unknown-unknown && \
    cd apps/kaskold-vault-web && \
    cargo "+$KASKOLD_REPRO_HOST_RUST" build --offline --locked --target wasm32-unknown-unknown --release

RUN source /etc/kaskold/toolchains.env && \
    cargo "+$KASKOLD_REPRO_HOST_RUST" build --offline --locked --manifest-path tools/Cargo.toml --bin gen-hash --release

# converge.sh <label> <output-basename> <board> <signed:0|1> <cargo arguments...>
# Five build/hash passes are mandatory. Generated hash/signature bytes live in
# flash rodata rather than executable code, so passes two through five must all
# agree. One final build is then made from the converged firmware_hash.rs. Board-specific
# partition policy is applied to every image-generation pass and final image.
RUN cat > /usr/local/bin/converge.sh <<'SCRIPT' && chmod +x /usr/local/bin/converge.sh
#!/bin/bash
set -euo pipefail
LABEL="$1"; shift
OUT="$1"; shift
BOARD="$1"; shift
SIGNED="$1"; shift
source /etc/kaskold/toolchains.env
source /root/esp-env.sh
cd /build/KasKold

python3 tools/build/firmware/board_layout.py check --board "${BOARD}"
mapfile -t BOARD_ESPFLASH_ARGS < <(
    python3 tools/build/firmware/board_layout.py espflash-args --board "${BOARD}"
)
FULL_FLASH_ARGS=("${BOARD_ESPFLASH_ARGS[@]}")
if [[ " ${FULL_FLASH_ARGS[*]} " != *" --flash-size "* ]]; then
    FULL_FLASH_ARGS+=(--flash-size 16mb)
fi

if [[ "${SIGNED}" == "1" ]]; then
    if [[ ! -f /run/secrets/signkey ]]; then
        echo "SKIPPED: ${LABEL} signed build — no signing key mounted"
        exit 0
    fi
    KEY_SIZE=$(stat -c '%s' /run/secrets/signkey)
    [[ "${KEY_SIZE}" == "32" ]] || {
        echo "BUILD FAILED: signing key must be exactly 32 bytes; got ${KEY_SIZE}"
        exit 1
    }
    KEY_ARGS=(/run/secrets/signkey)
    MODE=signed
else
    KEY_ARGS=()
    MODE=unsigned
fi

printf '\n%s\n' "${LABEL} (${MODE}) — five-pass convergence"
HASHES=()
for PASS in 1 2 3 4 5; do
    (cd apps/kaskold-hardware && cargo build --offline --locked --release "$@")
    PASS_IMAGE="/build/${OUT}-pass${PASS}.bin"
    espflash save-image --chip esp32s3 "${BOARD_ESPFLASH_ARGS[@]}" \
        apps/kaskold-hardware/target/xtensa-esp32s3-none-elf/release/kaskold-hardware \
        "${PASS_IMAGE}" 2>&1 | sed '/INFO/d'
    cargo "+$KASKOLD_REPRO_HOST_RUST" run --offline --locked --manifest-path tools/Cargo.toml --bin gen-hash --release -- \
        "${PASS_IMAGE}" "${KEY_ARGS[@]}" >/dev/null
    HASH=$(tools/build/firmware/build_with_hash.sh --read-generated-hash \
        apps/kaskold-hardware/src/firmware_hash.rs) || {
        echo "BUILD FAILED: failed to read generated EXPECTED_FIRMWARE_HASH"
        exit 1
    }
    HASHES+=("${HASH}")
    echo "  pass ${PASS}: ${HASH}"
done

if [[ "${HASHES[1]}" != "${HASHES[2]}" || "${HASHES[2]}" != "${HASHES[3]}" || "${HASHES[3]}" != "${HASHES[4]}" ]]; then
    echo "BUILD FAILED: ${LABEL} ${MODE} did not converge on passes 2 through 5"
    exit 1
fi

(cd apps/kaskold-hardware && cargo build --offline --locked --release "$@")
ELF=apps/kaskold-hardware/target/xtensa-esp32s3-none-elf/release/kaskold-hardware
espflash save-image --chip esp32s3 "${BOARD_ESPFLASH_ARGS[@]}" "${ELF}" "/build/${OUT}.bin" 2>&1 | sed '/INFO/d'
python3 tools/build/firmware/verify_image_hash.py \
    "/build/${OUT}.bin" apps/kaskold-hardware/src/firmware_hash.rs
espflash save-image --chip esp32s3 --merge "${FULL_FLASH_ARGS[@]}" "${ELF}" \
    "/build/${OUT}-full.bin" 2>&1 | sed '/INFO/d'
printf '%s\n' "${HASHES[4]}" > "/build/${OUT}.codehash"
rm -f "/build/${OUT}-pass"*.bin
echo "  CONVERGED: ${HASHES[4]}"
SCRIPT

# Unsigned builds come first so any verifier can reproduce the complete public
# comparison set without possessing the private release signing key. Production
# images execute boot-time known-answer tests; skip-tests is prohibited.
RUN --mount=type=secret,id=signkey,required=false \
    converge.sh "M5Stack" "kaskold-m5stack-unsigned" m5stack 0 \
        --no-default-features --features m5stack,production

RUN --mount=type=secret,id=signkey,required=false \
    converge.sh "M5Stack" "kaskold-m5stack" m5stack 1 \
        --no-default-features --features m5stack,production

# CoreS3 update manifests are intentionally not emitted by the normal release
# build; secure provisioning remains a separate operator-controlled flow.

# Assemble every release artifact and provenance manifest inside Docker. The
# host-side runner only asks BuildKit to export this directory; it does not
# compile firmware, calculate release hashes, or synthesize manifests itself.
RUN set -euo pipefail; \
    mkdir -p /release; \
    cp /build/kaskold-*.bin /release/; \
    cp /build/kaskold-*.codehash /release/; \
    find /build -maxdepth 1 -type f -name 'kaskold-*-update.ksfu' -exec cp {} /release/ \; ; \
    cp apps/kaskold-hardware/partitions/m5stack-cores3.csv /release/kaskold-m5stack-partitions.csv; \
    cp /build/SOURCE-SHA256SUMS /release/SOURCE-SHA256SUMS; \
    cp /opt/kaskold/input/BUILD-INPUT-SHA256SUMS /release/BUILD-INPUT-SHA256SUMS; \
    cp /opt/kaskold/input/BUILD-INPUT-MANIFEST.json /release/BUILD-INPUT-MANIFEST.json; \
    cd /release; \
    find . -maxdepth 1 -type f \( -name '*.bin' -o -name '*.codehash' -o -name '*.csv' -o -name '*.ksfu' \) \
        -printf '%f\n' \
        | sort \
        | xargs sha256sum \
        > SHA256SUMS; \
    source /etc/kaskold/toolchains.env; \
    HOST_RUST="$(rustc +"$KASKOLD_REPRO_HOST_RUST" --version)"; \
    ESP_RUST="$(source /root/esp-env.sh && rustc --version)"; \
    ESPFLASH_VERSION="$(source /root/esp-env.sh && espflash --version | head -1)"; \
    [[ "$ESPFLASH_VERSION" == *"$KASKOLD_ESPFLASH_VERSION"* ]] || { echo "BUILD FAILED: espflash version drift: $ESPFLASH_VERSION"; exit 1; }; \
    BUILD_COMMIT="${KASKOLD_GIT_COMMIT:-source-archive}"; \
    M5_PARTITION_SHA="$(sha256sum kaskold-m5stack-partitions.csv | awk '{print $1}')"; \
    SIGNED_IMAGES="$(find . -maxdepth 1 -type f -name 'kaskold-*.bin' ! -name '*-unsigned*' | wc -l | tr -d '[:space:]')"; \
    printf '%s\n' \
        'KasKold reproducible firmware build' \
        'format-version=2' \
        'builder=docker' \
        'platform=linux/amd64' \
        'toolchain-image=kaskold-toolchain:v3' \
        "source-date-epoch=${SOURCE_DATE_EPOCH}" \
        "build-commit=${BUILD_COMMIT}" \
        "host-rust=${HOST_RUST}" \
        "esp-rust=${ESP_RUST}" \
        "espflash=${ESPFLASH_VERSION}" \
        "espflash-policy=${KASKOLD_ESPFLASH_VERSION}" \
        'unsigned-images=2' \
        "signed-images=${SIGNED_IMAGES}" \
        'firmware-targets=m5stack' \
        'release-modes=app-only,full-flash' \
        'hash-convergence=5-pass;passes-2-through-5-must-match;identity-bytes=flash-rodata-static' \
        'final-codehash-verification=required;address+length+sha256' \
        'm5stack-partition-table=kaskold-m5stack-partitions.csv' \
        'm5stack-update-manifest=not-emitted-by-normal-release;secure-provisioning-is-separate' \
        "m5stack-partition-table-sha256=${M5_PARTITION_SHA}" \
        'm5stack-ota-apps=ota_0:0x10000+0x200000,ota_1:0x210000+0x200000' \
        'm5stack-persistent-state=offset:0xFFC000,size:0x4000' \
        'hardware-flashing=never' \
        > BUILD-MANIFEST.txt; \
    printf '{\n  "artifacts": [\n' > ARTIFACT-MANIFEST.json; \
    FIRST=1; \
    while IFS= read -r FILE; do \
        HASH="$(sha256sum "$FILE" | awk '{print $1}')"; \
        SIZE="$(stat -c '%s' "$FILE")"; \
        if [[ "$FIRST" == "0" ]]; then printf ',\n' >> ARTIFACT-MANIFEST.json; fi; \
        printf '    {"file":"%s","sha256":"%s","size":%s}' "$FILE" "$HASH" "$SIZE" >> ARTIFACT-MANIFEST.json; \
        FIRST=0; \
    done < <(find . -maxdepth 1 -type f \( -name '*.bin' -o -name '*.codehash' -o -name '*.csv' -o -name '*.ksfu' \) -printf '%f\n' | sort); \
    printf '\n  ],\n  "format_version": 1\n}\n' >> ARTIFACT-MANIFEST.json; \
    sha256sum ARTIFACT-MANIFEST.json BUILD-MANIFEST.txt SOURCE-SHA256SUMS \
        BUILD-INPUT-SHA256SUMS BUILD-INPUT-MANIFEST.json \
        > MANIFEST-SHA256SUMS

# BuildKit exports this stage directly to the caller-selected host directory.
# It contains only Docker-produced release artifacts and deterministic manifests.
FROM scratch AS artifacts
COPY --from=builder /release/ /

# Preserve the historical inspectable image target for maintainers who build
# Dockerfile manually without --target artifacts.
FROM builder AS image
CMD bash -c '\
    echo "KasKold reproducible build outputs"; \
    echo "-- code-segment hashes --"; \
    for f in /release/*.codehash; do printf "%-42s %s\n" "$(basename "$f")" "$(cat "$f")"; done; \
    echo "-- image SHA-256 hashes --"; \
    cat /release/SHA256SUMS'
