import importlib.util
import pathlib
import sys
import tempfile
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "qa/checks"))


class PopItSecureBootTests(unittest.TestCase):


    def test_patcher_refuses_unknown_idf_source_and_gates_expected_blocks(self):
        path = ROOT / "tools/build/firmware/secure_bootloader/m5stack/patch_pop_it_bootloader.py"
        spec = importlib.util.spec_from_file_location("pop_it_patcher", path)
        module = importlib.util.module_from_spec(spec)
        assert spec.loader is not None
        spec.loader.exec_module(module)
        fixture = (
            "#include <string.h>\n"
            "static bool ota_has_initial_contents;\n"
            + module.ANTI_ROLLBACK_FUNCTION_ANCHOR
            + "}\n"
            + module.LOAD_BOOT_ANCHOR
            + "}\n"
            "static void load_image(const esp_image_metadata_t *image_data) {\n"
            + module.SECURE_BOOT_BLOCK
            + "#ifdef CONFIG_SECURE_FLASH_ENC_ENABLED\n"
            + "    bool flash_encryption_enabled = esp_flash_encrypt_state();\n"
            + module.FLASH_ENCRYPTION_MUTATION_ANCHOR + " }\n"
            + module.FLASH_ENCRYPTION_MUTATION_ANCHOR + " }\n"
            + module.FLASH_ENCRYPTION_COMMIT_ANCHOR + "\n"
            + "#endif\n"
            + "}\n"
        )
        digest = bytes(range(32))
        with tempfile.TemporaryDirectory() as td:
            source = pathlib.Path(td) / "bootloader_utility.c"
            source.write_text(fixture)
            module.patch_utility(source, digest)
            patched = source.read_text()
            self.assertEqual(patched.count("KasKold owner-authority boot-control and OTA handoff"), 1)
            self.assertIn("kaskold_process_owner_boot_control(bs, start_index)", patched)
            self.assertIn("kaskold_pop_it_transition_armed", patched)
            self.assertEqual(patched.count("!flash_encryption_enabled && kaskold_pop_it_transition_armed"), 2)
            take = patched.index("kaskold_bootctl_take(KASKOLD_BOOTCTL_OP_POP_IT")
            burn = patched.index("esp_secure_boot_v2_permanently_enable(image_data)")
            self.assertLess(take, burn)
            self.assertIn("anti-rollback eFuse update deferred until Pop It", patched)
            self.assertIn("0x00, 0x01, 0x02, 0x03", patched)
            with self.assertRaises(SystemExit):
                module.patch_utility(source, digest)
        with tempfile.TemporaryDirectory() as td:
            source = pathlib.Path(td) / "bootloader_utility.c"
            source.write_text("static bool ota_has_initial_contents;\n")
            with self.assertRaises(SystemExit):
                module.patch_utility(source, digest)

    def test_bootloader_preflight_and_owner_install_bind_exact_authority_keys(self):
        path = ROOT / "tools/build/firmware/secure_bootloader/m5stack/patch_pop_it_bootloader.py"
        spec = importlib.util.spec_from_file_location("pop_it_patcher_preflight", path)
        module = importlib.util.module_from_spec(spec)
        assert spec.loader is not None
        spec.loader.exec_module(module)
        fixture = (
            "static esp_err_t s_calculate_image_public_key_digests(void);\n"
            + module.SECURE_BOOT_PREFLIGHT_ANCHOR
            + "{ return ESP_OK; }\n"
        )
        digest = bytes(range(32))
        with tempfile.TemporaryDirectory() as td:
            source = pathlib.Path(td) / "secure_boot.c"
            source.write_text(fixture)
            module.patch_secure_boot(source, digest)
            patched = source.read_text()
            self.assertIn("kaskold_expected_provisioning_sbv2_digest", patched)
            self.assertIn("kaskold_secure_boot_v2_verify_key_digest", patched)
            self.assertIn("s_calculate_image_public_key_digests", patched)
            self.assertIn("bootloader is not signed by the expected official key", patched)
            self.assertIn("application is not signed by the expected official key", patched)
            self.assertIn("image_len - SIG_BLOCK_PADDING", patched)
            with self.assertRaises(SystemExit):
                module.patch_secure_boot(source, digest)
        with tempfile.TemporaryDirectory() as td:
            source = pathlib.Path(td) / "secure_boot.c"
            source.write_text(fixture)
            with self.assertRaises(SystemExit):
                module.patch_secure_boot(source, b"too short")

    def test_bootloader_owner_only_render_closes_vendor_authority_slots(self):
        path = ROOT / "tools/build/firmware/secure_bootloader/m5stack/patch_pop_it_bootloader.py"
        spec = importlib.util.spec_from_file_location("pop_it_patcher_owner_only", path)
        module = importlib.util.module_from_spec(spec)
        assert spec.loader is not None
        spec.loader.exec_module(module)
        digest = bytes(range(32))
        utility_fixture = (
            "static bool ota_has_initial_contents;\n"
            + module.ANTI_ROLLBACK_FUNCTION_ANCHOR + "}\n"
            + module.LOAD_BOOT_ANCHOR + "}\n"
            "static void load_image(const esp_image_metadata_t *image_data) {\n"
            + module.SECURE_BOOT_BLOCK
            + "#ifdef CONFIG_SECURE_FLASH_ENC_ENABLED\n"
            + "    bool flash_encryption_enabled = esp_flash_encrypt_state();\n"
            + module.FLASH_ENCRYPTION_MUTATION_ANCHOR + " }\n"
            + module.FLASH_ENCRYPTION_MUTATION_ANCHOR + " }\n"
            + module.FLASH_ENCRYPTION_COMMIT_ANCHOR + "\n#endif\n}\n"
        )
        secure_fixture = (
            "static esp_err_t s_calculate_image_public_key_digests(void);\n"
            + module.SECURE_BOOT_PREFLIGHT_ANCHOR + "{ return ESP_OK; }\n"
        )
        with tempfile.TemporaryDirectory() as td:
            utility = pathlib.Path(td) / "bootloader_utility.c"
            secure = pathlib.Path(td) / "secure_boot.c"
            utility.write_text(utility_fixture)
            secure.write_text(secure_fixture)
            module.patch_utility(utility, digest, True)
            module.patch_secure_boot(secure, digest, True)
            utility_text = utility.read_text()
            secure_text = secure.read_text()
            self.assertIn("#define KASKOLD_OWNER_ONLY_AUTHORITY 1", utility_text)
            self.assertIn("#define KASKOLD_OWNER_ONLY_AUTHORITY 1", secure_text)
            self.assertIn("esp_efuse_set_digest_revoke(1)", utility_text)
            self.assertIn("esp_efuse_set_digest_revoke(2)", utility_text)
            self.assertIn("owner-only authority must be enrolled before provisioning", utility_text)
            self.assertIn("expected owner-only key", secure_text)


if __name__ == "__main__":
    unittest.main()
