use {monedero_store::KvStorage, wasm_bindgen_test::*};
wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn local_storage() -> anyhow::Result<()> {
    let key = "key";
    let value = String::from("value");
    let kv = KvStorage::new();
    kv.set(key, value.clone())?;
    let stored: Option<String> = kv.get(key)?;
    assert!(stored.is_some());
    assert_eq!(value, stored.unwrap());

    kv.delete(key)?;
    let not_found: Option<String> = kv.get(key)?;
    assert!(not_found.is_none());

    // make sure delete doesn't throw error
    kv.delete(key)?;

    kv.set(key, value.clone())?;
    kv.clear();
    let not_found: Option<String> = kv.get(key)?;
    assert!(not_found.is_none());
    Ok(())
}
