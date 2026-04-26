pub mod bundle;
pub mod cli;
pub mod init_scaffold;
pub mod pack_ops;
pub mod rojo_utils;

use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use rbx_dom_weak::{InstanceBuilder, WeakDom};

pub fn make_dom(scripts: &[(&str, &str, &str)]) -> WeakDom {
    let root = scripts.iter().fold(
        InstanceBuilder::new("DataModel"),
        |root, (name, class, source)| {
            let mut builder = InstanceBuilder::new(*class).with_name(*name);
            if matches!(*class, "LocalScript" | "ModuleScript" | "Script") {
                builder = builder.with_property("Source", *source);
            }
            root.with_child(builder)
        },
    );

    WeakDom::new(root)
}

pub fn write_rbxm(dir: &Path, name: &str, dom: &WeakDom) -> PathBuf {
    let path = dir.join(format!("{name}.rbxm"));
    let file = File::create(&path).expect("failed to create rbxm file");
    let writer = BufWriter::new(file);

    rbx_binary::to_writer(writer, dom, dom.root().children()).expect("failed to write rbxm");

    path
}

pub fn fixture_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(relative)
}
