"""Secure Boot v2 source templates for KasKold CoreS3 authority profiles.

Kept separate from the OTA/boot-control template so both security modules remain
small enough for direct review and the repository module-size policy.
"""
from __future__ import annotations

SECURE_BOOT_PREFLIGHT_ANCHOR = (
    "static esp_err_t check_and_generate_secure_boot_keys(const esp_image_metadata_t *image_data)\n"
)
SECURE_BOOT_BLOCK = """#ifdef CONFIG_SECURE_BOOT_V2_ENABLED
    err = esp_secure_boot_v2_permanently_enable(image_data);
    if (err != ESP_OK) {
        ESP_LOGE(TAG, \"Secure Boot v2 failed (%d)\", err);
        return;
    }
#endif
"""


SECURE_BOOT_HELPER_TEMPLATE = r'''

/* KasKold exact-key Secure Boot v2 verification helpers.
 *
 * s_calculate_image_public_key_digests() validates each present signature
 * block (image digest, block CRC and RSA-PSS signature) before returning the
 * public-key digest.  These helpers additionally bind that valid signature to
 * one exact authority digest.
 */
#define KASKOLD_OWNER_ONLY_AUTHORITY __KASKOLD_OWNER_ONLY__

static const uint8_t kaskold_expected_provisioning_sbv2_digest[ESP_SECURE_BOOT_KEY_DIGEST_LEN] = {
__KASKOLD_EXPECTED_DIGEST__
};

static bool kaskold_digest_set_contains(
    const esp_image_sig_public_key_digests_t *digests,
    const uint8_t expected[ESP_SECURE_BOOT_KEY_DIGEST_LEN])
{
    for (unsigned i = 0; i < digests->num_digests; ++i) {
        if (memcmp(digests->key_digests[i], expected,
                   ESP_SECURE_BOOT_KEY_DIGEST_LEN) == 0) {
            return true;
        }
    }
    return false;
}

esp_err_t kaskold_secure_boot_v2_verify_key_digest(
    uint32_t flash_offset,
    uint32_t image_len,
    const uint8_t expected[ESP_SECURE_BOOT_KEY_DIGEST_LEN])
{
    if (expected == NULL || image_len <= SIG_BLOCK_PADDING) {
        return ESP_ERR_INVALID_ARG;
    }
    esp_image_sig_public_key_digests_t digests = { 0 };
    esp_err_t ret = s_calculate_image_public_key_digests(
        flash_offset, image_len - SIG_BLOCK_PADDING, &digests);
    if (ret != ESP_OK || digests.num_digests == 0) {
        return ret == ESP_OK ? ESP_ERR_IMAGE_INVALID : ret;
    }
    return kaskold_digest_set_contains(&digests, expected) ? ESP_OK : ESP_ERR_IMAGE_INVALID;
}

esp_err_t kaskold_secure_boot_v2_authority_preflight(const esp_image_metadata_t *image_data)
{
    if (image_data == NULL || esp_secure_boot_enabled()) {
        return ESP_ERR_INVALID_STATE;
    }

    esp_image_metadata_t bootloader_data = { 0 };
    esp_err_t ret = esp_image_verify_bootloader_data(&bootloader_data);
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "KasKold: installed bootloader image is invalid (%d)", ret);
        return ret;
    }
    ret = kaskold_secure_boot_v2_verify_key_digest(
        bootloader_data.start_addr,
        bootloader_data.image_len,
        kaskold_expected_provisioning_sbv2_digest);
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, KASKOLD_OWNER_ONLY_AUTHORITY
                 ? "KasKold: bootloader is not signed by the expected owner-only key"
                 : "KasKold: bootloader is not signed by the expected official key");
        return ret;
    }

    ret = kaskold_secure_boot_v2_verify_key_digest(
        image_data->start_addr,
        image_data->image_len,
        kaskold_expected_provisioning_sbv2_digest);
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, KASKOLD_OWNER_ONLY_AUTHORITY
                 ? "KasKold: application is not signed by the expected owner-only key"
                 : "KasKold: application is not signed by the expected official key");
        return ret;
    }
    ESP_LOGI(TAG, KASKOLD_OWNER_ONLY_AUTHORITY
             ? "KasKold: bootloader/app signatures match the expected owner-only key"
             : "KasKold: bootloader/app signatures match the expected official key");
    return ESP_OK;
}
'''


POP_IT_GATED_BLOCK = r'''#ifdef CONFIG_SECURE_BOOT_V2_ENABLED
    bool kaskold_pop_it_transition_armed = false;
    if (esp_secure_boot_enabled()) {
        ESP_LOGI(TAG, "KasKold Pop It: Secure Boot v2 already hardware enforced");
    } else {
        kaskold_bootctl_record_t record = { 0 };
        if (kaskold_bootctl_read(&record) && record.operation == KASKOLD_BOOTCTL_OP_POP_IT) {
            err = kaskold_secure_boot_v2_authority_preflight(image_data);
            if (err != ESP_OK) {
                ESP_LOGE(TAG, "KasKold Pop It: configured-authority/image preflight refused (%d)", err);
                return;
            }
#if KASKOLD_OWNER_ONLY_AUTHORITY
            if (!kaskold_authority_state_matches(kaskold_expected_provisioning_sbv2_digest)) {
                ESP_LOGE(TAG, "KasKold Pop It: owner-only authority must be enrolled before provisioning");
                return;
            }
#endif
            /* Consent only arms provisioning here. No eFuse is touched until
             * the flash-encryption block below runs under this same request. */
            kaskold_pop_it_transition_armed = true;
            ESP_LOGI(TAG, "KasKold Pop It: explicit provisioning request accepted");
        } else {
            ESP_LOGI(TAG, "KasKold Pop It: all irreversible provisioning deferred by user");
        }
    }
#endif
'''

POP_IT_COMMIT_BLOCK = r'''#ifdef CONFIG_SECURE_BOOT_V2_ENABLED
    if (kaskold_pop_it_transition_armed) {
        kaskold_bootctl_record_t record = { 0 };
#ifdef CONFIG_SECURE_FLASH_ENC_ENABLED
        if (!esp_efuse_is_flash_encryption_enabled()) {
            ESP_LOGE(TAG, "KasKold Pop It: flash encryption was not enabled; refusing Secure Boot burn");
            return;
        }
#endif
        if (!kaskold_bootctl_take(KASKOLD_BOOTCTL_OP_POP_IT, &record)) {
            ESP_LOGE(TAG, "KasKold Pop It: consent request could not be consumed");
            return;
        }
#if SOC_SUPPORTS_SECURE_DL_MODE
        err = esp_efuse_enable_rom_secure_download_mode();
        if (err != ESP_OK) {
            ESP_LOGE(TAG, "KasKold Pop It: Secure Download mode provisioning refused (%d)", err);
            return;
        }
#endif
        err = esp_secure_boot_v2_permanently_enable(image_data);
        if (err != ESP_OK) {
            ESP_LOGE(TAG, "KasKold Pop It: Secure Boot v2 provisioning refused (%d)", err);
            return;
        }
        ESP_LOGI(TAG, "KasKold Pop It: irreversible provisioning completed after explicit consent");
    }
#endif
'''
