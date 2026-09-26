cmake_minimum_required(VERSION 3.22.1)
project(kaskold_vault_jni LANGUAGES CXX)

if(NOT DEFINED KASKOLD_REPOSITORY_ROOT)
    message(FATAL_ERROR "KASKOLD_REPOSITORY_ROOT must point to the KasKold repository root")
endif()

if(ANDROID_ABI STREQUAL "arm64-v8a")
    set(RUST_TARGET "aarch64-linux-android")
elseif(ANDROID_ABI STREQUAL "x86_64")
    set(RUST_TARGET "x86_64-linux-android")
else()
    message(FATAL_ERROR "Unsupported KasKold Vault ABI: ${ANDROID_ABI}")
endif()

set(VAULT_RUNTIME_A "${KASKOLD_REPOSITORY_ROOT}/target/${RUST_TARGET}/release/libvault_runtime.a")
if(NOT EXISTS "${VAULT_RUNTIME_A}")
    message(FATAL_ERROR "Missing Rust vault-runtime static library: ${VAULT_RUNTIME_A}")
endif()

add_library(vault_runtime STATIC IMPORTED)
set_target_properties(vault_runtime PROPERTIES IMPORTED_LOCATION "${VAULT_RUNTIME_A}")

add_library(vault_runtime_jni SHARED vault_jni.cpp)
target_compile_features(vault_runtime_jni PRIVATE cxx_std_17)
target_link_libraries(vault_runtime_jni PRIVATE vault_runtime log android dl m)
