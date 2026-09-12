use anyhow::Result;
use astrolabe::{
    model::{
        AssetRef, Selection,
        io::{Content, Load, Save},
    },
    rete::{self, Store},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Node {
    name: String,
    children: Vec<AssetRef<Node>>,
    exact: [f32; 3],
    #[serde(with = "serde_bytes")]
    data: Vec<u8>,
}
impl Content for Node {
    const KIND: &'static str = "test-node";
    fn validate(&self) -> Result<()> {
        anyhow::ensure!(!self.name.is_empty(), "empty node");
        Ok(())
    }
}
fn node(name: &str) -> Node {
    Node {
        name: name.into(),
        children: vec![],
        exact: [-0.0, 1.25, -12.5],
        data: (0..=255).collect(),
    }
}

#[test]
fn content_graph_identity_and_level_sessions() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let mut store = Store::new(temp.path(), Selection::hype("a", "en")?)?;
    let shared = AssetRef::<Node>::new("fix:/models/shared")?;
    let equal_but_distinct = AssetRef::<Node>::new("fix:/models/distinct")?;
    let root = AssetRef::<Node>::new("level:/scene/root")?;
    let mut parent = node("scene");
    parent.children = vec![shared.clone(), shared.clone(), equal_but_distinct.clone()];
    // Forward references: save the root before its dependencies.
    store.save(&root, &parent)?;
    store.save(&shared, &node("same values"))?;
    store.save(&equal_but_distinct, &node("same values"))?;
    let loaded = store.load(&root)?;
    assert_eq!(*loaded, parent);
    let x = store.load(&loaded.children[0])?;
    let y = store.load(&loaded.children[1])?;
    let z = store.load(&loaded.children[2])?;
    assert!(Arc::ptr_eq(&x, &y));
    assert!(!Arc::ptr_eq(&x, &z));
    assert_eq!(*x, *z);
    assert_eq!(loaded.exact[0].to_bits(), (-0.0_f32).to_bits());
    store.select(Selection::hype("b", "en")?)?;
    assert!(store.load(&root).is_err());
    store.save(&root, &node("level b"))?;
    store.select(Selection::hype("a", "en")?)?;
    assert_eq!(store.load(&root)?.name, "scene");
    assert!(!Arc::ptr_eq(&x, &store.load(&shared)?));
    Ok(())
}

#[test]
fn messagepack_binary_and_version_validation() -> Result<()> {
    let n = node("bytes");
    let bytes = rete::encode(&n)?;
    // serde_bytes makes this a bin16 field, not an array of 256 tagged integers.
    assert!(bytes.windows(3).any(|x| x == [0xc5, 0x01, 0x00]));
    assert_eq!(rete::decode::<Node>(&bytes)?, n);
    let mut bad = bytes.clone();
    let offset = bad.windows(7).position(|x| x == b"schema\x01").unwrap();
    bad[offset + 6] = 2;
    assert!(rete::decode::<Node>(&bad).is_err());
    let mut trailing = bytes;
    trailing.push(0);
    assert!(rete::decode::<Node>(&trailing).is_err());
    for invalid in [
        "level:/../secret",
        "level://root",
        "level:/x\\y",
        "file:/tmp/x",
        "fix:/x#",
    ] {
        assert!(AssetRef::<Node>::new(invalid).is_err(), "{invalid}");
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
struct AlternateNode(Node);
impl Content for AlternateNode {
    const KIND: &'static str = "alternate-test-node";
    fn validate(&self) -> Result<()> {
        self.0.validate()
    }
}

#[derive(Serialize)]
struct TestEnvelope<'a> {
    format: &'a str,
    schema: u32,
    kind: &'a str,
    value: &'a Node,
}

#[test]
fn envelope_kind_magic_schema_values_and_exact_floats_are_checked() -> Result<()> {
    let valid = node("valid");
    for (format, schema, kind) in [
        ("not-rete", 1, Node::KIND),
        ("astrolabe-rete", 0, Node::KIND),
        ("astrolabe-rete", 2, Node::KIND),
        ("astrolabe-rete", 1, AlternateNode::KIND),
    ] {
        let bytes = rmp_serde::to_vec_named(&TestEnvelope {
            format,
            schema,
            kind,
            value: &valid,
        })?;
        assert!(rete::decode::<Node>(&bytes).is_err());
    }
    let invalid = node("");
    assert!(rete::encode(&invalid).is_err());
    let bytes = rmp_serde::to_vec_named(&TestEnvelope {
        format: "astrolabe-rete",
        schema: 1,
        kind: Node::KIND,
        value: &invalid,
    })?;
    assert!(
        rete::decode::<Node>(&bytes).is_err(),
        "load must validate semantic content too"
    );
    for bits in [0x8000_0000, 0x7fc1_2345, 0xff80_0000, 0x0000_0001] {
        let mut value = valid.clone();
        value.exact[0] = f32::from_bits(bits);
        let loaded: Node = rete::decode(&rete::encode(&value)?)?;
        assert_eq!(
            loaded.exact[0].to_bits(),
            bits,
            "MessagePack changed exact float bits"
        );
    }
    Ok(())
}

#[test]
fn typed_kind_checks_cover_cached_disk_and_replaced_content() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let mut store = Store::new(temp.path(), Selection::hype("astrolabe", "EN")?)?;
    let reference = AssetRef::<Node>::new("level:/scene/root")?;
    let other = AssetRef::<AlternateNode>::new("level:/scene/root")?;
    store.save(&reference, &node("old"))?;
    let old = store.load(&reference)?;
    assert!(
        store.load(&other).is_err(),
        "typed mismatch on cached content"
    );
    store.select(Selection::hype("astrolabe", "EN")?)?;
    assert!(
        store.load(&other).is_err(),
        "typed mismatch in persisted envelope"
    );
    store.save(&reference, &node("edited"))?;
    let edited = store.load(&reference)?;
    assert_eq!(old.name, "old");
    assert_eq!(edited.name, "edited");
    assert!(!Arc::ptr_eq(&old, &edited));
    store.save(&other, &AlternateNode(node("new kind")))?;
    assert!(
        store.load(&reference).is_err(),
        "saving a different kind must invalidate old cached value"
    );
    assert_eq!(store.load(&other)?.0.name, "new kind");
    let member = AssetRef::<Node>::new("level:/scene/root#child/0")?;
    assert!(store.load(&member).is_err());
    assert!(store.save(&member, &node("member")).is_err());
    Ok(())
}

#[test]
fn shared_asset_level_links_rebind_a_b_a_and_resolver_is_target_independent() -> Result<()> {
    use astrolabe::model::content::Collection;
    fn observe<L: Load<Node>>(
        loader: &mut L,
        reference: &AssetRef<Node>,
    ) -> Result<(String, String)> {
        let shared = loader.load(reference)?;
        let current = loader.load(&shared.children[0])?;
        Ok((shared.name.clone(), current.name.clone()))
    }
    let temp = tempfile::tempdir()?;
    let mut store = Store::new(temp.path(), Selection::hype("a", "EN")?)?;
    let fix = AssetRef::<Node>::new("fix:/family/shared")?;
    let level = AssetRef::<Node>::new("level:/scene/current")?;
    let mut shared = node("shared family");
    shared.children.push(level.clone());
    store.save(&fix, &shared)?;
    store.save(&level, &node("level a"))?;
    let held = store.load(&fix)?;
    assert_eq!(
        observe(&mut store, &fix)?,
        ("shared family".into(), "level a".into())
    );
    store.select(Selection::hype("b", "EN")?)?;
    store.save(&level, &node("level b"))?;
    assert_eq!(
        observe(&mut store, &fix)?,
        ("shared family".into(), "level b".into())
    );
    assert!(!Arc::ptr_eq(&held, &store.load(&fix)?));
    store.select(Selection::hype("a", "EN")?)?;
    assert_eq!(
        observe(&mut store, &fix)?,
        ("shared family".into(), "level a".into())
    );
    // The same consumer traverses useful content from direct in-memory import.
    let mut memory = Collection::default();
    memory.insert(&fix, shared)?;
    memory.insert(&level, node("level a"))?;
    assert_eq!(observe(&mut memory, &fix)?, observe(&mut store, &fix)?);
    Ok(())
}

#[test]
fn package_roots_paths_and_selection_are_validated() -> Result<()> {
    use astrolabe::{
        model::AssetPath,
        rete::{Package, Root},
    };
    let temp = tempfile::tempdir()?;
    let selection = Selection::hype("astrolabe", "EN")?;
    let mut store = Store::new(temp.path(), selection.clone())?;
    let root = AssetRef::<Node>::new("level:/scene/root")?;
    store.save(&root, &node("root"))?;
    let package = Package {
        diagnostics: vec![],
        selection: selection.clone(),
        roots: vec![Root {
            kind: Node::KIND.into(),
            reference: root.path.clone(),
        }],
    };
    store.save_package(&package)?;
    let (mut reopened, loaded) = Store::open(temp.path())?;
    assert_eq!(loaded, package);
    assert_eq!(reopened.load(&root)?.name, "root");
    let mut bad = package.clone();
    bad.roots.push(bad.roots[0].clone());
    assert!(rete::encode(&bad).is_err(), "duplicate package root");
    let mut bad = package.clone();
    bad.roots[0].kind.clear();
    assert!(rete::encode(&bad).is_err(), "empty package root kind");
    let mut bad = package.clone();
    bad.roots[0].reference = AssetPath::new("level:/scene/root#child")?;
    assert!(
        rete::encode(&bad).is_err(),
        "root must load an owning asset"
    );
    let mut other_selection = package.clone();
    other_selection.selection.level = "different".into();
    assert!(store.save_package(&other_selection).is_err());
    for level in ["", ".", "..", "../x", "a/b", "a\\b", "a:b", "a\0b"] {
        assert!(
            Selection::hype(level, "EN").is_err(),
            "invalid level {level:?}"
        );
    }
    for locale in ["", ".", "..", "a/b", "a\\b", "a:b", "a\0b"] {
        assert!(
            Selection::hype("astrolabe", locale).is_err(),
            "invalid locale {locale:?}"
        );
    }
    for text in [
        "fix:/../x",
        "fix:/x/../y",
        "level:/x#",
        "level:/x#a#b",
        "level:/x\0y",
        "sound:/x\\y",
        "file:/tmp/x",
    ] {
        let serialized = rmp_serde::to_vec(text)?;
        assert!(
            rmp_serde::from_slice::<AssetPath>(&serialized).is_err(),
            "deserialization bypassed path validation: {text:?}"
        );
    }
    Ok(())
}
