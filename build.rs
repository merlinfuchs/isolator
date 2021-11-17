use std::env;
use std::path::{Path, PathBuf};
use deno_core::{JsRuntime, RuntimeOptions};

fn build_protobuf() {
    tonic_build::configure()
        .build_client(false)
        .build_server(true)
        .compile(&["./protobuf/service.proto"], &["./protobuf"])
        .unwrap();
}

fn get_js_files(dir: &str) -> Vec<PathBuf> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut js_files = std::fs::read_dir(dir)
        .unwrap()
        .map(|dir_entry| {
            let file = dir_entry.unwrap();
            manifest_dir.join(file.path())
        })
        .filter(|path| path.extension().unwrap_or_default() == "js")
        .collect::<Vec<PathBuf>>();
    js_files.sort();
    js_files
}

fn create_runtime() -> JsRuntime {
    let extensions = vec![
        ext_webidl::init(),
        ext_web::init(),
        ext_timers::init(),
        ext_resources::init(),
        ext_console::init()
    ];

    let mut runtime = JsRuntime::new(RuntimeOptions {
        will_snapshot: true,
        extensions,
        ..Default::default()
    });

    let display_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    for js_file in get_js_files("js") {
        let display_path = js_file.strip_prefix(display_root).unwrap();
        let display_path_str = display_path.display().to_string();
        println!("cargo:rerun-if-changed={}", display_path_str);
        runtime
            .execute_script(
                &("isolator:".to_string() + &display_path_str.replace('\\', "/")),
                &std::fs::read_to_string(&js_file).unwrap(),
            )
            .unwrap();
    }

    runtime
}

fn build_runtime_snapshot() {
    let mut runtime = create_runtime();

    let snapshot = runtime.snapshot();
    let snapshot_slice: &[u8] = &*snapshot;

    let o = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let snapshot_path = o.join("RUNTIME_SNAPSHOT.bin");

    std::fs::write(&snapshot_path, snapshot_slice).unwrap();
}

fn main() {
    build_protobuf();
    build_runtime_snapshot()
}