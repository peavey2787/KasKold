from __future__ import annotations

from pathlib import Path


def nested_struct_expectations(data_root: Path) -> dict[Path, dict[str, tuple[str, ...]]]:
    return {
        data_root / "wallet.rs": {
            "SeedSession": ("seed_mgr", "mnemonic_indices", "word_count", "active_source", "seed_loaded", "seed_list_scroll", "pending_delete_slot", "pending_add_wallet_kind", "pending_wallet_name", "pending_wallet_name_len", "pending_add_wallet_slot", "pending_multisig_wallet_key", "pending_wallet_protection", "pending_wallet_activation_salt", "pending_wallet_activation_verifier", "pending_bip39_passphrase", "pending_bip39_passphrase_len", "dice_collector", "pending_seed_entropy", "pending_seed_entropy_valid", "touch_collector", "word_input", "pp_input", "bip85_index", "bip85_child_indices"),
            "KeyMaterialState": ("acct_key_raw", "hex_input", "hex_input_len"),
            "AddressState": ("current_addr_index", "pubkey_cache", "change_pubkey_cache", "view_is_change", "partial_redraw", "pubkeys_cached", "extra_pubkey", "extra_pubkey_index", "extra_change_pubkey", "extra_change_pubkey_index", "input_buf", "input_len", "cache_seed_derivation", "cache_worker_generation", "cache_progress", "cache_started_at_ms", "cache_last_progress_at_ms"),
        },
        data_root / "qr.rs": {
            "OutgoingQrState": ("purpose", "manual_frames", "buffer", "length", "frame", "frame_count", "covenant_backup_length", "close_state"),
            "QrPresentationState": ("large", "mode", "via_density"),
            "QrScanState": ("address", "address_length", "address_valid", "address_hidden_key"),
        },
        data_root / "storage.rs": {
            "FileBrowserState": ("file_list", "file_count", "file_scroll", "selected_file", "text_import_kind"),
            "ExportFileState": ("filename", "overwrite_prompt", "overwrite_prompt_len", "encrypted_operation"),
            "StorageConfirmationState": ("overwrite_action", "overwrite_back", "delete_return"),
            "TextFileList": ("file_names", "display_names", "display_lens", "file_count"),
        },
        data_root / "signing.rs": {
            "TransactionSigningState": ("active", "signatures_present", "signatures_required", "input_format", "pskt_parsed", "output_ownership", "initial_signature_counts"),
            "MultisigState": ("store", "creating", "threshold", "participant_count", "scroll", "descriptor_indices", "descriptor_count", "descriptor_persisted", "descriptor_save_prompted", "picking_key"),
            "MessageSigningState": ("payload", "payload_len", "signature", "hash"),
            "CommitRevealState": ("plaintext", "plaintext_len", "hash", "ciphertext"),
        },
        data_root / "signing/anti_klepto.rs": {
            "AntiKleptoSigningState": ("phase", "session_id", "host_commitment", "transaction_digest", "initial_sig_counts"),
        },
        data_root / "stego.rs": {
            "StegoSessionState": ("result_ok", "auto_scan", "portable"),
            "StegoExportState": ("carrier", "security", "portable_confirmation_digest", "portable_confirmation_pending", "jpeg_file_names", "jpeg_display_names", "jpeg_display_lens", "jpeg_file_count", "jpeg_selected", "jpeg_desc_buf", "jpeg_desc_len"),
            "StegoHintState": ("buffer", "length"),
            "StegoImportState": ("descriptor_buf", "descriptor_len", "jpeg_names", "jpeg_display", "jpeg_display_lens", "jpeg_count", "jpeg_selected", "carrier", "embedded_payload", "embedded_payload_len", "recovered_hint", "recovered_hint_len"),
        },
    }


# (field prefix in source, error label, forbidden direct field names)
LEGACY_DOMAIN_FIELDS: tuple[tuple[str, str, tuple[str, ...]], ...] = (
    ("wallet", "wallet state", ("seed_mgr", "mnemonic_indices", "word_count", "seed_loaded", "pp_input", "acct_key_raw", "pubkey_cache", "current_addr_index", "hex_input")),
    ("qr", "QR state", ("signed_qr_buf", "signed_qr_len", "signed_qr_frame", "signed_qr_nframes", "covb_len", "scanned_addr")),
    ("storage", "storage state", ("sd_file_list", "sd_file_count", "sd_file_scroll", "sd_selected_file", "txt_import_type", "kspt_filename", "kspt_encrypt", "sd_overwrite_next", "sd_overwrite_back", "sd_delete_return", "sd_txt_origin")),
    ("stego", "stego state", ("stego_mode_idx", "stego_result_ok", "stego_auto_scan", "jpeg_file_names", "txt_file_names", "stego_pp_buf", "import_jpeg_names", "import_exif_b64")),
)
