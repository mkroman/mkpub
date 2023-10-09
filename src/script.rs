//! Scripting support

use std::path::PathBuf;

use rhai::plugin::*;
use rhai::{CustomType, Engine, ImmutableString, TypeBuilder};
use tracing::{instrument, trace};

/// The default Rhai program to use when the user hasn't provided their own.
pub const DEFAULT_RHAI_PROGRAM: &str = include_str!("../program.rhai");

#[export_module]
mod path_module {
    /// Returns the file name for the given `path`.
    #[rhai_fn(get = "file_name")]
    #[allow(clippy::ptr_arg)]
    pub fn get_file_name(path: &mut PathBuf) -> ImmutableString {
        path.file_name().unwrap().to_str().unwrap().into()
    }
}

#[export_module]
mod mime_module {
    /// Guesses and returns the mime type as a string based on the given `file_name`.
    pub fn guess_from_path(path: PathBuf) -> ImmutableString {
        let mime = mime_guess::from_path(path).first_or_octet_stream();

        mime.essence_str().into()
    }
}

#[derive(Debug, Clone, Default)]
pub struct ScriptablePutObject {
    pub content_type: Option<String>,
    pub content_disposition: Option<String>,
    pub key: Option<String>,
}

impl ScriptablePutObject {
    pub fn set_content_type(&mut self, value: ImmutableString) {
        self.content_type = Some(value.into_owned());
    }

    pub fn set_content_disposition(&mut self, value: ImmutableString) {
        self.content_disposition = Some(value.into_owned());
    }

    pub fn get_key(&mut self) -> ImmutableString {
        self.key.clone().unwrap_or_default().into()
    }

    pub fn set_key(&mut self, value: ImmutableString) {
        self.key = Some(value.into_owned());
    }
}

impl CustomType for ScriptablePutObject {
    fn build(mut builder: TypeBuilder<Self>) {
        builder
            .with_name("Object")
            .with_get_set("key", Self::get_key, Self::set_key)
            .with_set("content_type", Self::set_content_type)
            .with_set("content_disposition", Self::set_content_disposition)
            .on_print(|v| format!("Object({:?})", v.key))
            .on_debug(|v| format!("Object({v:?})"));
    }
}

#[instrument]
pub fn build_engine() -> Engine {
    trace!("constructing scripting engine");
    let mut engine = Engine::new();

    trace!("registering custom types");
    engine.build_type::<ScriptablePutObject>();

    trace!("registering modules");
    {
        let module = exported_module!(path_module);
        engine.register_global_module(module.into());
    }

    {
        let module = exported_module!(mime_module);
        engine.register_static_module("mime", module.into());
    }

    trace!("finished scripting engine");
    engine
}
