mod parser;
pub(crate) mod utils;
mod walk_data;

use std::sync::{Arc, LazyLock};

use parser::DefineParserPlugin;
use rspack_core::{
  Compilation, CompilationId, CompilationParams, CompilerCompilation, ModuleType,
  NormalModuleFactoryParser, ParserAndGenerator, ParserOptions, Plugin,
};
use rspack_error::{Diagnostic, Error, Result};
use rspack_hook::{plugin, plugin_hook};
use rspack_util::fx_hash::{FxDashMap, FxHashMap};
use serde_json::Value;

use self::{utils::code_to_string, walk_data::WalkData};
use crate::parser_and_generator::JavaScriptParserAndGenerator;

pub(crate) const VALUE_DEP_PREFIX: &str = "rspack/DefinePlugin ";
pub(crate) const IMPORT_META_ENV_VALUE_DEP_KEY: &str = "rspack/DefinePlugin import.meta.env.*";

#[derive(Debug)]
struct ConflictingValuesError(String, String, String);

impl ConflictingValuesError {
  fn into_diagnostic(self) -> Diagnostic {
    Error::warning(format!(
      "DefinePlugin:\nConflicting values for '{}' ({} !== {})",
      self.0, self.1, self.2
    ))
    .into()
  }
}

pub type DefineValue = FxHashMap<String, Value>;
pub(crate) type ImportMetaEnvDefinitions = FxHashMap<String, Value>;

#[derive(Debug, Default)]
struct ImportMetaEnvDefinitionsState {
  definitions: ImportMetaEnvDefinitions,
  serialized: Option<String>,
}

static IMPORT_META_ENV_DEFINITIONS_MAP: LazyLock<
  FxDashMap<CompilationId, ImportMetaEnvDefinitionsState>,
> = LazyLock::new(Default::default);

#[plugin]
#[derive(Debug)]
pub struct DefinePlugin {
  walk_data: Arc<WalkData>,
}

impl DefinePlugin {
  pub fn new(definitions: DefineValue) -> Self {
    Self::new_inner(Arc::new(WalkData::new(&definitions)))
  }
}

pub(crate) fn serialize_import_meta_env_definitions(
  definitions: &ImportMetaEnvDefinitions,
) -> String {
  let mut pairs = definitions
    .iter()
    .map(|(key, value)| (key.as_str(), code_to_string(value, None, None)))
    .collect::<Vec<_>>();
  pairs.sort_unstable_by(|a, b| a.0.cmp(b.0));
  let content = pairs
    .into_iter()
    .map(|(key, value)| format!("{}:{value}", rspack_util::json_stringify_str(key)))
    .collect::<Vec<_>>()
    .join(",");
  format!("{{{content}}}")
}

pub(crate) fn remove_import_meta_env_definitions(compilation_id: CompilationId) {
  IMPORT_META_ENV_DEFINITIONS_MAP.remove(&compilation_id);
}

pub(crate) fn import_meta_env_definitions_string(compilation_id: CompilationId) -> String {
  IMPORT_META_ENV_DEFINITIONS_MAP
    .get(&compilation_id)
    .map_or_else(
      || "{}".to_string(),
      |state| {
        state
          .serialized
          .clone()
          .unwrap_or_else(|| serialize_import_meta_env_definitions(&state.definitions))
      },
    )
}

pub(crate) fn has_import_meta_env_definition(compilation_id: CompilationId, name: &str) -> bool {
  IMPORT_META_ENV_DEFINITIONS_MAP
    .get(&compilation_id)
    .is_some_and(|state| state.definitions.contains_key(name))
}

#[plugin_hook(CompilerCompilation for DefinePlugin, tracing=false)]
async fn collect_import_meta_env_definitions(
  &self,
  compilation: &mut Compilation,
  _params: &mut CompilationParams,
) -> Result<()> {
  compilation.extend_diagnostics(self.walk_data.diagnostics.clone());
  let mut definitions = IMPORT_META_ENV_DEFINITIONS_MAP
    .entry(compilation.id())
    .or_default();
  for (key, value) in &self.walk_data.import_meta_env_definitions {
    definitions.definitions.insert(key.clone(), value.clone());
    definitions.serialized = None;
  }
  for (key, value) in self.walk_data.tiling_definitions.iter() {
    let cache_key = format!("{VALUE_DEP_PREFIX}{key}");
    if let Some(prev) = compilation.value_cache_versions.get(&cache_key)
      && prev != value
    {
      compilation.push_diagnostic(
        ConflictingValuesError(key.clone(), prev.clone(), value.clone()).into_diagnostic(),
      );
    } else {
      compilation
        .value_cache_versions
        .insert(cache_key, value.clone());
    }
  }

  Ok(())
}

#[plugin_hook(CompilerCompilation for DefinePlugin, stage = i32::MAX, tracing=false)]
async fn finalize_import_meta_env_definitions(
  &self,
  compilation: &mut Compilation,
  _params: &mut CompilationParams,
) -> Result<()> {
  let mut state = IMPORT_META_ENV_DEFINITIONS_MAP
    .entry(compilation.id())
    .or_default();
  let serialized = match &state.serialized {
    Some(serialized) => serialized.clone(),
    None => {
      let serialized = serialize_import_meta_env_definitions(&state.definitions);
      state.serialized = Some(serialized.clone());
      serialized
    }
  };
  compilation
    .value_cache_versions
    .insert(IMPORT_META_ENV_VALUE_DEP_KEY.to_string(), serialized);

  Ok(())
}

#[plugin_hook(NormalModuleFactoryParser for DefinePlugin, tracing=false)]
async fn nmf_parser(
  &self,
  module_type: &ModuleType,
  parser: &mut Box<dyn ParserAndGenerator>,
  _parser_options: Option<&ParserOptions>,
) -> Result<()> {
  if module_type.is_js_like()
    && let Some(parser) = parser.downcast_mut::<JavaScriptParserAndGenerator>()
  {
    parser.add_parser_plugin(Box::new(DefineParserPlugin::new(self.walk_data.clone())));
  }
  Ok(())
}

impl Plugin for DefinePlugin {
  fn name(&self) -> &'static str {
    "rspack.DefinePlugin"
  }

  fn clear_cache(&self, id: CompilationId) {
    remove_import_meta_env_definitions(id);
  }

  fn apply(&self, ctx: &mut rspack_core::ApplyContext<'_>) -> Result<()> {
    ctx
      .compiler_hooks
      .compilation
      .tap(collect_import_meta_env_definitions::new(self));
    ctx
      .compiler_hooks
      .compilation
      .tap(finalize_import_meta_env_definitions::new(self));
    ctx
      .normal_module_factory_hooks
      .parser
      .tap(nmf_parser::new(self));
    Ok(())
  }
}
