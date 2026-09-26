#include <jni.h>
#include <cstdint>
#include <ctime>
#include <cstdlib>
#include <string>
#include <sys/types.h>
#include <vector>

extern "C" {
void* kaskold_vault_new();
void kaskold_vault_destroy(void*);
int32_t kaskold_vault_create(void*, uint32_t);
int32_t kaskold_vault_restore(void*, const uint8_t*, size_t, const uint8_t*, size_t);
void kaskold_vault_lock(void*);
int32_t kaskold_vault_export_public_account(void*);
int32_t kaskold_vault_backup_words(void*);
int32_t kaskold_vault_backup_seedqr(void*, uint8_t);
int32_t kaskold_vault_backup_xprv(void*);
int32_t kaskold_vault_export_receive_key(void*, uint32_t);
int32_t kaskold_vault_seal(void*, const uint8_t*, size_t);
int32_t kaskold_vault_unlock_sealed(void*, const uint8_t*, size_t, const uint8_t*, size_t);
int32_t kaskold_vault_begin_scan(void*);
int32_t kaskold_vault_accept_frame(void*, const uint8_t*, size_t);
int32_t kaskold_vault_approve(void*, uint64_t);
void kaskold_vault_reject(void*);
size_t kaskold_vault_response_count(void*);
ssize_t kaskold_vault_response_frame_copy(void*, size_t, uint8_t*, size_t);
ssize_t kaskold_vault_last_text_copy(void*, uint8_t*, size_t);
ssize_t kaskold_vault_last_bytes_copy(void*, uint8_t*, size_t);
ssize_t kaskold_vault_last_error_copy(void*, uint8_t*, size_t);
int32_t kaskold_vault_workflow_text(void*, const uint8_t*, size_t, const uint8_t*, size_t);
int32_t kaskold_vault_workflow_bytes(void*, const uint8_t*, size_t, const uint8_t*, size_t, const uint8_t*, size_t);
int32_t kaskold_vault_workflow_text_with_bytes(void*, const uint8_t*, size_t, const uint8_t*, size_t, const uint8_t*, size_t);
}

namespace {
using Handle = void*;

void wipe(std::vector<uint8_t>& bytes) {
    volatile uint8_t* p = bytes.data();
    for (size_t i = 0; i < bytes.size(); ++i) p[i] = 0;
}

std::vector<uint8_t> copy_byte_array(JNIEnv* env, jbyteArray value) {
    if (value == nullptr) return {};
    const jsize len = env->GetArrayLength(value);
    std::vector<uint8_t> out(static_cast<size_t>(len));
    if (len > 0) env->GetByteArrayRegion(value, 0, len, reinterpret_cast<jbyte*>(out.data()));
    return out;
}

template <typename Copier>
std::vector<uint8_t> copy_native(Copier copier) {
    const ssize_t probe = copier(nullptr, 0);
    if (probe >= 0) return {};
    const size_t required = static_cast<size_t>(-probe);
    std::vector<uint8_t> out(required);
    const ssize_t written = copier(out.data(), out.size());
    if (written < 0 || static_cast<size_t>(written) != required) return {};
    return out;
}

std::vector<uint8_t> last_error(Handle handle) {
    return copy_native([&](uint8_t* dst, size_t cap) { return kaskold_vault_last_error_copy(handle, dst, cap); });
}

void throw_native(JNIEnv* env, Handle handle, const char* fallback) {
    std::vector<uint8_t> error = last_error(handle);
    std::string message = error.empty() ? fallback : std::string(error.begin(), error.end());
    wipe(error);
    jclass type = env->FindClass("java/lang/IllegalStateException");
    if (type != nullptr) env->ThrowNew(type, message.c_str());
}

jstring last_text(JNIEnv* env, Handle handle) {
    std::vector<uint8_t> bytes = copy_native([&](uint8_t* dst, size_t cap) { return kaskold_vault_last_text_copy(handle, dst, cap); });
    std::string text(bytes.begin(), bytes.end());
    wipe(bytes);
    return env->NewStringUTF(text.c_str());
}

jbyteArray last_bytes(JNIEnv* env, Handle handle) {
    std::vector<uint8_t> bytes = copy_native([&](uint8_t* dst, size_t cap) { return kaskold_vault_last_bytes_copy(handle, dst, cap); });
    jbyteArray result = env->NewByteArray(static_cast<jsize>(bytes.size()));
    if (result != nullptr && !bytes.empty()) {
        env->SetByteArrayRegion(result, 0, static_cast<jsize>(bytes.size()), reinterpret_cast<const jbyte*>(bytes.data()));
    }
    wipe(bytes);
    return result;
}

Handle handle_from(jlong value) { return reinterpret_cast<Handle>(static_cast<uintptr_t>(value)); }
jlong handle_to(Handle value) { return static_cast<jlong>(reinterpret_cast<uintptr_t>(value)); }
}

extern "C" JNIEXPORT jlong JNICALL
Java_com_kaskold_vault_RuntimeBridge_nativeCreateRuntime(JNIEnv*, jclass) {
    return handle_to(kaskold_vault_new());
}

extern "C" JNIEXPORT void JNICALL
Java_com_kaskold_vault_RuntimeBridge_nativeDestroyRuntime(JNIEnv*, jclass, jlong handle) {
    kaskold_vault_destroy(handle_from(handle));
}

extern "C" JNIEXPORT jstring JNICALL
Java_com_kaskold_vault_RuntimeBridge_nativeCreateWallet(JNIEnv* env, jclass, jlong raw, jint words) {
    Handle handle = handle_from(raw);
    if (kaskold_vault_create(handle, static_cast<uint32_t>(words)) != 0) {
        throw_native(env, handle, "Wallet creation failed"); return nullptr;
    }
    return last_text(env, handle);
}

extern "C" JNIEXPORT jstring JNICALL
Java_com_kaskold_vault_RuntimeBridge_nativeRestoreWallet(JNIEnv* env, jclass, jlong raw, jbyteArray phraseArray, jbyteArray passphraseArray) {
    Handle handle = handle_from(raw);
    auto phrase = copy_byte_array(env, phraseArray);
    auto passphrase = copy_byte_array(env, passphraseArray);
    const int32_t rc = kaskold_vault_restore(handle, phrase.data(), phrase.size(), passphrase.data(), passphrase.size());
    wipe(phrase); wipe(passphrase);
    if (rc != 0) { throw_native(env, handle, "Wallet restore failed"); return nullptr; }
    return last_text(env, handle);
}

extern "C" JNIEXPORT jbyteArray JNICALL
Java_com_kaskold_vault_RuntimeBridge_nativeSealWallet(JNIEnv* env, jclass, jlong raw, jbyteArray keyArray) {
    Handle handle = handle_from(raw);
    auto key = copy_byte_array(env, keyArray);
    const int32_t rc = kaskold_vault_seal(handle, key.data(), key.size());
    wipe(key);
    if (rc != 0) { throw_native(env, handle, "Wallet sealing failed"); return nullptr; }
    return last_bytes(env, handle);
}

extern "C" JNIEXPORT jstring JNICALL
Java_com_kaskold_vault_RuntimeBridge_nativeUnlockSealedWallet(JNIEnv* env, jclass, jlong raw, jbyteArray sealedArray, jbyteArray keyArray) {
    Handle handle = handle_from(raw);
    auto sealed = copy_byte_array(env, sealedArray);
    auto key = copy_byte_array(env, keyArray);
    const int32_t rc = kaskold_vault_unlock_sealed(handle, sealed.data(), sealed.size(), key.data(), key.size());
    wipe(sealed); wipe(key);
    if (rc != 0) { throw_native(env, handle, "Wallet unlock failed"); return nullptr; }
    return last_text(env, handle);
}

extern "C" JNIEXPORT jstring JNICALL
Java_com_kaskold_vault_RuntimeBridge_nativeExportPublicAccount(JNIEnv* env, jclass, jlong raw) {
    Handle handle = handle_from(raw);
    if (kaskold_vault_export_public_account(handle) != 0) { throw_native(env, handle, "Public-account export failed"); return nullptr; }
    return last_text(env, handle);
}

extern "C" JNIEXPORT jstring JNICALL
Java_com_kaskold_vault_RuntimeBridge_nativeBackupWords(JNIEnv* env, jclass, jlong raw) {
    Handle handle = handle_from(raw);
    if (kaskold_vault_backup_words(handle) != 0) { throw_native(env, handle, "Recovery-word backup failed"); return nullptr; }
    return last_text(env, handle);
}

extern "C" JNIEXPORT jbyteArray JNICALL
Java_com_kaskold_vault_RuntimeBridge_nativeBackupSeedQr(JNIEnv* env, jclass, jlong raw, jboolean compact) {
    Handle handle = handle_from(raw);
    if (kaskold_vault_backup_seedqr(handle, compact == JNI_TRUE ? 1 : 0) != 0) { throw_native(env, handle, "SeedQR backup failed"); return nullptr; }
    return last_bytes(env, handle);
}

extern "C" JNIEXPORT jstring JNICALL
Java_com_kaskold_vault_RuntimeBridge_nativeBackupXprv(JNIEnv* env, jclass, jlong raw) {
    Handle handle = handle_from(raw);
    if (kaskold_vault_backup_xprv(handle) != 0) { throw_native(env, handle, "XPrv backup failed"); return nullptr; }
    return last_text(env, handle);
}

extern "C" JNIEXPORT jstring JNICALL
Java_com_kaskold_vault_RuntimeBridge_nativeExportReceiveKey(JNIEnv* env, jclass, jlong raw, jint index) {
    Handle handle = handle_from(raw);
    if (index < 0 || kaskold_vault_export_receive_key(handle, static_cast<uint32_t>(index)) != 0) {
        throw_native(env, handle, "Private-key export failed"); return nullptr;
    }
    return last_text(env, handle);
}

extern "C" JNIEXPORT void JNICALL
Java_com_kaskold_vault_RuntimeBridge_nativeBeginScan(JNIEnv* env, jclass, jlong raw) {
    Handle handle = handle_from(raw);
    if (kaskold_vault_begin_scan(handle) != 0) throw_native(env, handle, "QR scan initialization failed");
}

extern "C" JNIEXPORT jstring JNICALL
Java_com_kaskold_vault_RuntimeBridge_nativeAcceptQrFrame(JNIEnv* env, jclass, jlong raw, jbyteArray frameArray) {
    Handle handle = handle_from(raw);
    auto frame = copy_byte_array(env, frameArray);
    const int32_t rc = kaskold_vault_accept_frame(handle, frame.data(), frame.size());
    wipe(frame);
    if (rc < 0) { throw_native(env, handle, "QR frame was rejected"); return nullptr; }
    return last_text(env, handle);
}

extern "C" JNIEXPORT jobjectArray JNICALL
Java_com_kaskold_vault_RuntimeBridge_nativeApprove(JNIEnv* env, jclass, jlong raw) {
    Handle handle = handle_from(raw);
    const uint64_t now_unix = static_cast<uint64_t>(std::time(nullptr));
    if (kaskold_vault_approve(handle, now_unix) != 0) { throw_native(env, handle, "Transaction approval failed"); return nullptr; }
    const size_t count = kaskold_vault_response_count(handle);
    jclass byteArrayClass = env->FindClass("[B");
    jobjectArray result = env->NewObjectArray(static_cast<jsize>(count), byteArrayClass, nullptr);
    for (size_t i = 0; i < count; ++i) {
        const ssize_t probe = kaskold_vault_response_frame_copy(handle, i, nullptr, 0);
        if (probe >= 0) continue;
        std::vector<uint8_t> frame(static_cast<size_t>(-probe));
        const ssize_t written = kaskold_vault_response_frame_copy(handle, i, frame.data(), frame.size());
        if (written < 0) { wipe(frame); throw_native(env, handle, "Signed response extraction failed"); return nullptr; }
        jbyteArray item = env->NewByteArray(static_cast<jsize>(frame.size()));
        env->SetByteArrayRegion(item, 0, static_cast<jsize>(frame.size()), reinterpret_cast<const jbyte*>(frame.data()));
        env->SetObjectArrayElement(result, static_cast<jsize>(i), item);
        env->DeleteLocalRef(item);
        wipe(frame);
    }
    return result;
}

extern "C" JNIEXPORT void JNICALL
Java_com_kaskold_vault_RuntimeBridge_nativeReject(JNIEnv*, jclass, jlong raw) { kaskold_vault_reject(handle_from(raw)); }

extern "C" JNIEXPORT void JNICALL
Java_com_kaskold_vault_RuntimeBridge_nativeLock(JNIEnv*, jclass, jlong raw) { kaskold_vault_lock(handle_from(raw)); }


extern "C" JNIEXPORT jstring JNICALL
Java_com_kaskold_vault_RuntimeBridge_nativeWorkflowText(JNIEnv* env, jclass, jlong raw, jbyteArray operationArray, jbyteArray inputArray) {
    Handle handle = handle_from(raw);
    auto operation = copy_byte_array(env, operationArray);
    auto input = copy_byte_array(env, inputArray);
    const int32_t rc = kaskold_vault_workflow_text(handle, operation.data(), operation.size(), input.data(), input.size());
    wipe(operation); wipe(input);
    if (rc != 0) { throw_native(env, handle, "Vault workflow failed"); return nullptr; }
    return last_text(env, handle);
}

extern "C" JNIEXPORT jbyteArray JNICALL
Java_com_kaskold_vault_RuntimeBridge_nativeWorkflowBytes(JNIEnv* env, jclass, jlong raw, jbyteArray operationArray, jbyteArray inputArray, jbyteArray dataArray) {
    Handle handle = handle_from(raw);
    auto operation = copy_byte_array(env, operationArray);
    auto input = copy_byte_array(env, inputArray);
    auto data = copy_byte_array(env, dataArray);
    const int32_t rc = kaskold_vault_workflow_bytes(handle, operation.data(), operation.size(), input.data(), input.size(), data.data(), data.size());
    wipe(operation); wipe(input); wipe(data);
    if (rc != 0) { throw_native(env, handle, "Vault byte workflow failed"); return nullptr; }
    return last_bytes(env, handle);
}


extern "C" JNIEXPORT jstring JNICALL
Java_com_kaskold_vault_RuntimeBridge_nativeWorkflowTextWithBytes(JNIEnv* env, jclass, jlong raw, jbyteArray operationArray, jbyteArray inputArray, jbyteArray dataArray) {
    Handle handle = handle_from(raw);
    auto operation = copy_byte_array(env, operationArray);
    auto input = copy_byte_array(env, inputArray);
    auto data = copy_byte_array(env, dataArray);
    const int32_t rc = kaskold_vault_workflow_text_with_bytes(handle, operation.data(), operation.size(), input.data(), input.size(), data.data(), data.size());
    wipe(operation); wipe(input); wipe(data);
    if (rc != 0) { throw_native(env, handle, "Vault byte-input workflow failed"); return nullptr; }
    return last_text(env, handle);
}
