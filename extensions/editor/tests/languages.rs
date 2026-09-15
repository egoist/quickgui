use quickgui_extension_sdk::{ComponentFactory, WakeHandle, abi, schema};
use serde_json::json;
use std::{
    ffi::c_void,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    time::Duration,
};

unsafe extern "C" fn wake(context: *mut c_void) {
    unsafe { &*context.cast::<Arc<AtomicUsize>>() }.fetch_add(1, Ordering::SeqCst);
}
unsafe extern "C" fn release_wake(context: *mut c_void) {
    drop(unsafe { Box::from_raw(context.cast::<Arc<AtomicUsize>>()) });
}
struct Reply {
    sender: mpsc::Sender<(u32, String)>,
    releases: Arc<AtomicUsize>,
}
unsafe extern "C" fn emit(context: *mut c_void, kind: u32, value: abi::Bytes) {
    let reply = unsafe { &*context.cast::<Reply>() };
    let text = std::str::from_utf8(unsafe { std::slice::from_raw_parts(value.data, value.len) })
        .unwrap()
        .to_owned();
    reply.sender.send((kind, text)).unwrap();
}
unsafe extern "C" fn release(context: *mut c_void) {
    let reply = unsafe { Box::from_raw(context.cast::<Reply>()) };
    reply.releases.fetch_add(1, Ordering::SeqCst);
}
fn call(api: abi::ServiceApi, path: &str) -> (u32, String) {
    let (sender, receiver) = mpsc::channel();
    let releases = Arc::new(AtomicUsize::new(0));
    let reply = Box::into_raw(Box::new(Reply {
        sender,
        releases: releases.clone(),
    }))
    .cast();
    let params = serde_json::to_vec(&json!({"path":path})).unwrap();
    unsafe {
        (api.invoke)(
            1,
            abi::Bytes::new(b"load-language-pack"),
            abi::Bytes::new(&params),
            abi::ServiceSink {
                context: reply,
                emit,
                release,
            },
        )
    };
    let result = receiver
        .recv_timeout(Duration::from_secs(10))
        .expect("language service did not reply");
    // Channel disconnect proves the worker released its owned sink after the one reply.
    assert!(matches!(
        receiver.recv_timeout(Duration::from_secs(10)),
        Err(mpsc::RecvTimeoutError::Disconnected)
    ));
    assert_eq!(releases.load(Ordering::SeqCst), 1);
    result
}
fn spans(node: &schema::Node) -> usize {
    node.spans
        .iter()
        .filter(|span| span.foreground.is_some())
        .count()
        + node.children.iter().map(spans).sum::<usize>()
}

#[test]
fn wasm_pack_loads_and_refreshes_an_existing_editor_without_changing_its_source() {
    let Ok(path) = std::env::var("QUICKGUI_TEST_LANGUAGE_PACK") else {
        eprintln!("Run bun scripts/check-language-packs.ts for the native-pack integration test");
        return;
    };
    let descriptor = unsafe { &*quickgui_editor::quickgui_extension_v1() };
    assert_eq!(descriptor.kind, abi::PACKAGE_EXTENSION);
    let api = unsafe { *descriptor.api.cast::<abi::PackageApi>() };
    let wakes = Arc::new(AtomicUsize::new(0));
    let owned = Box::into_raw(Box::new(wakes.clone())).cast();
    let wake = unsafe {
        WakeHandle::from_raw(abi::Wake {
            context: owned,
            wake,
            release: release_wake,
        })
    };
    let source = "local answer = 42";
    let mut editor=quickgui_editor::Factory::create("editor",json!({"value":source,"language":"lua","syntaxTheme":{"keyword":"#ff0000","number":"#00ff00","function":"#0000ff"}}),wake.clone()).unwrap();
    assert_eq!(
        spans(&editor.render(Default::default()).unwrap().nodes[0]),
        0
    );
    let result = call(api.service, &path);
    assert_eq!(result, (0, r#"["lua","rust"]"#.into()));
    assert!(wakes.load(Ordering::SeqCst) > 0);
    let frame = editor.render(Default::default()).unwrap();
    assert_eq!(frame.nodes[0].text, source);
    assert!(spans(&frame.nodes[0]) >= 2);
    let count = wakes.load(Ordering::SeqCst);
    assert_eq!(call(api.service, &path), result);
    assert_eq!(
        wakes.load(Ordering::SeqCst),
        count,
        "duplicate loads must not invalidate surfaces"
    );
    assert_eq!(
        quickgui_editor::SyntaxLanguage::from_name("lua-fence"),
        quickgui_editor::SyntaxLanguage::from_path("script.lua")
    );
    assert_eq!(
        call(api.service, "/nonexistent/quickgui-language-pack.json").0,
        1
    );
    assert_eq!(call(api.service, "relative.json").0, 1);
    fn render_colors(component: &mut dyn quickgui_extension_sdk::Component) -> usize {
        fn lists(node: &schema::Node, out: &mut Vec<(u64, usize)>) {
            if let Some(list) = &node.list {
                out.push((list.renderer, list.count));
            }
            for child in &node.children {
                lists(child, out);
            }
        }
        let frame = component.render(Default::default()).unwrap();
        let mut requests = Vec::new();
        let mut colors = 0;
        for node in &frame.nodes {
            colors += spans(node);
            lists(node, &mut requests);
        }
        for (renderer, count) in requests {
            for node in component
                .render(schema::RenderRequest {
                    renderer: Some(renderer),
                    start: 0,
                    end: count.min(16),
                    ..Default::default()
                })
                .unwrap()
                .nodes
            {
                colors += spans(&node);
            }
        }
        colors
    }
    for name in ["code-block", "diff-view"] {
        let props = json!({"value":source,"language":"lua-fence","oldPath":"test.lua","newPath":"test.lua","oldText":"local answer = 1","newText":source,"syntaxTheme":{"keyword":"#ff0000","number":"#00ff00"}});
        let mut component = quickgui_editor::Factory::create(name, props, wake.clone()).unwrap();
        assert!(
            render_colors(component.as_mut()) >= 2,
            "{name} did not use the registered grammar"
        );
    }

    use std::io::Read;
    let original = std::fs::read(&path).unwrap();
    let mut archive = tar::Archive::new(std::io::Cursor::new(&original));
    let mut members = Vec::new();
    for entry in archive.entries().unwrap() {
        let mut entry = entry.unwrap();
        let name = entry.path().unwrap().to_string_lossy().into_owned();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        members.push((name, bytes));
    }
    let manifest = members
        .iter()
        .position(|(name, _)| name == "manifest.json")
        .unwrap();
    let mut bad: serde_json::Value = serde_json::from_slice(&members[manifest].1).unwrap();
    for (index, language) in bad["languages"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        language["name"] = json!(format!("rollback-language-{index}"));
        language["aliases"] = json!([]);
        language["extensions"] = json!([]);
        language["filenames"] = json!([]);
    }
    bad["languages"][1]["queries"]["highlights"] = json!("(nonexistent_node) @keyword");
    members[manifest].1 = serde_json::to_vec(&bad).unwrap();
    let mut builder = tar::Builder::new(Vec::new());
    for (name, bytes) in members {
        let mut header = tar::Header::new_ustar();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, name, bytes.as_slice())
            .unwrap();
    }
    let bad = builder.into_inner().unwrap();
    let before = quickgui_editor::syntax_language_generation();
    assert!(quickgui_editor::load_syntax_language_pack_bytes(&bad).is_err());
    assert!(quickgui_editor::SyntaxLanguage::from_name("rollback-language-0").is_none());
    assert_eq!(
        quickgui_editor::syntax_language_generation(),
        before,
        "failed packs must be atomic"
    );
}
