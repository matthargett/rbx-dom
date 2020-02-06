mod api_dump;
mod database;
mod property_patches;
mod run_in_roblox;

use std::{
    borrow::Cow,
    collections::HashMap,
    error::Error,
    fs::{self, File},
    io::{self, BufWriter, Write},
    path::PathBuf,
    str,
};

use rbx_dom_weak::{RbxInstanceProperties, RbxTree, RbxValue};
use rbx_reflection::Scriptability;
use rbx_types::Variant;
use serde_derive::Deserialize;

use crate::{
    api_dump::Dump,
    database::ReflectionDatabase,
    property_patches::load_property_patches,
    run_in_roblox::{inject_plugin_main, run_in_roblox},
};

static PLUGIN_MAIN: &str = include_str!("../plugin/main.lua");

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum PluginMessage {
    Version {
        version: [u32; 4],
    },

    #[serde(rename_all = "camelCase")]
    PatchDescriptors {
        class_name: String,
        descriptors: HashMap<Cow<'static, str>, DescriptorPatch>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DescriptorPatch {
    default_value: Option<Variant>,
    scriptability: Option<Scriptability>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut database = ReflectionDatabase::new();

    let dump = Dump::read()?;
    database.populate_from_dump(&dump)?;

    let property_patches = load_property_patches();
    database.populate_from_patches(&property_patches)?;

    database.validate();

    let plugin = create_plugin(&database);
    let messages = run_in_roblox(&plugin);

    process_plugin_messages(&mut database, &messages);
    emit_database(&database)?;

    Ok(())
}

fn emit_database(database: &ReflectionDatabase) -> io::Result<()> {
    let rust_output = {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.pop();
        path.push("rbx_reflection_database/database.msgpack");
        path
    };

    let lua_output = {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.pop();
        path.push("rbx_dom_lua/src/database-json.txt");
        path
    };

    Ok(())
}

fn process_plugin_messages(database: &mut ReflectionDatabase, messages: &[Vec<u8>]) {
    for message in messages {
        let deserialized = match serde_json::from_slice(&message) {
            Ok(v) => v,
            Err(e) => {
                panic!(
                    "Couldn't deserialize message: {}\n{}",
                    e,
                    str::from_utf8(message).unwrap()
                );
            }
        };

        match deserialized {
            PluginMessage::Version { version } => {
                database.studio_version = version;
            }
            PluginMessage::PatchDescriptors {
                class_name,
                descriptors,
            } => {
                if let Some(class) = database.classes.get_mut(class_name.as_str()) {
                    for (property_name, patch) in descriptors {
                        if let Some(default_value) = patch.default_value {
                            class
                                .default_properties
                                .insert(property_name.clone(), default_value.clone());
                        }

                        if let Some(descriptor) = class.properties.get_mut(&property_name) {
                            if let Some(scriptability) = patch.scriptability {
                                descriptor.scriptability = scriptability;
                            }
                        }
                    }
                }
            }
        }
    }
}

fn create_plugin(database: &ReflectionDatabase) -> RbxTree {
    let mut plugin = RbxTree::new(RbxInstanceProperties {
        name: String::from("generate_reflection plugin"),
        class_name: String::from("Folder"),
        properties: Default::default(),
    });

    let root_id = plugin.get_root_id();

    let mut main_properties = HashMap::new();
    main_properties.insert(
        String::from("Source"),
        RbxValue::String {
            value: PLUGIN_MAIN.to_owned(),
        },
    );

    let main = RbxInstanceProperties {
        name: String::from("Main"),
        class_name: String::from("ModuleScript"),
        properties: main_properties,
    };

    plugin.insert_instance(main, root_id);

    inject_plugin_main(&mut plugin);
    inject_reflection_classes(&mut plugin, &database);
    inject_dependencies(&mut plugin);

    plugin
}

fn create_module(name: &str, source: String) -> RbxInstanceProperties {
    let mut properties = HashMap::new();

    properties.insert(String::from("Source"), RbxValue::String { value: source });

    RbxInstanceProperties {
        class_name: String::from("ModuleScript"),
        name: String::from(name),
        properties,
    }
}

fn inject_reflection_classes(plugin: &mut RbxTree, database: &ReflectionDatabase) {
    let root_id = plugin.get_root_id();

    let mut classes_buffer = Vec::new();
    // emitter_lua::emit_classes(&mut classes_buffer, database)
    //     .expect("Couldn't emit Lua class database");

    let classes_source =
        String::from_utf8(classes_buffer).expect("Lua class database wasn't valid UTF-8");

    let module = create_module("ReflectionClasses", classes_source);
    plugin.insert_instance(module, root_id);
}

/// Injects in the pieces of rbx_dom_lua that we need to generate the dump.
///
/// Notably, this reduces the code duplication for serializing/deserializing
/// values through JSON. We manually track dependencies right now to avoid
/// needing to depend on Rojo to build the plugin.
fn inject_dependencies(plugin: &mut RbxTree) {
    static BASE64: &str = include_str!("../../rbx_dom_lua/src/base64.lua");
    static ENCODED_VALUE: &str = include_str!("../../rbx_dom_lua/src/EncodedValue.lua");

    let root_id = plugin.get_root_id();

    plugin.insert_instance(create_module("base64", String::from(BASE64)), root_id);
    plugin.insert_instance(
        create_module("EncodedValue", String::from(ENCODED_VALUE)),
        root_id,
    );
}
