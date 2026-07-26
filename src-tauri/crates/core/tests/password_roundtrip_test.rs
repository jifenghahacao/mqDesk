//! 密码存储往返测试
//!
//! 复现：create_connection 保存密码后，connect_to 无法 load_password
//! 的问题（Windows keyring 在 Tauri dev 下不稳定）。

use mqdesk_core::storage::Storage;

#[test]
fn password_save_load_roundtrip() {
    let tmp = tempfile::tempdir().expect("创建临时目录失败");
    let storage = Storage::open(tmp.path().to_path_buf()).expect("打开 sled 失败");

    let id = "test-conn-id";
    let password = "guest-secret";

    storage.save_password(id, password).expect("保存密码应成功");
    let loaded = storage.load_password(id).expect("读取密码应成功");

    assert_eq!(loaded, password, "读取的密码应与保存的一致");
}
