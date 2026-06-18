mod parser;
pub(crate) mod utils;
mod walk_data;

use std::sync::Arc;

use parser::DefineParserPlugin;
use rspack_core::{
  Compilation, CompilationParams, CompilerCompilation, ModuleType, NormalModuleFactoryParser,
  ParserAndGenerator, ParserOptions, Plugin,
};
use rspack_error::{Diagnostic, Error, Result};
use rspack_hook::{plugin, plugin_hook};
use rustc_hash::FxHashMap;
use serde_json::Value;

use self::walk_data::WalkData;
use crate::parser_and_generator::JavaScriptParserAndGenerator;

pub(crate) const VALUE_DEP_PREFIX: &str = "rspack/DefinePlugin ";
pub(crate) const IMPORT_META_ENV_VALUE_DEP_KEY: &str = "rspack/DefinePlugin import.meta.env.*";
const IMPORT_META_ENV_PREFIX: &str = "import.meta.env.";

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

#[plugin]
#[derive(Debug)]
pub struct DefinePlugin {
  walk_data: Arc<WalkData>,
  definitions: Arc<DefineValue>,
}

impl DefinePlugin {
  pub fn new(definitions: DefineValue) -> Self {
    Self::new_inner(Arc::new(WalkData::new(&definitions)), Arc::new(definitions))
  }
}

fn import_meta_env_value(definitions: &DefineValue) -> String {
  let mut pairs = definitions
    .iter()
    .filter_map(|(key, value)| {
      key
        .strip_prefix(IMPORT_META_ENV_PREFIX)
        .map(|env_key| (env_key, value.to_string()))
    })
    .collect::<Vec<_>>();
  pairs.sort_unstable_by(|a, b| a.0.cmp(b.0));
  pairs
    .into_iter()
    .map(|(key, value)| format!("{key}:{value}"))
    .collect::<Vec<_>>()
    .join(",")
}

#[plugin_hook(CompilerCompilation for DefinePlugin, tracing=false)]
async fn compilation(
  &self,
  compilation: &mut Compilation,
  _params: &mut CompilationParams,
) -> Result<()> {
  compilation.extend_diagnostics(self.walk_data.diagnostics.clone());
  compilation.define_plugin_definitions.extend(
    self
      .definitions
      .iter()
      .map(|(key, value)| (key.clone(), value.clone())),
  );
  compilation.value_cache_versions.insert(
    IMPORT_META_ENV_VALUE_DEP_KEY.to_string(),
    import_meta_env_value(&compilation.define_plugin_definitions),
  );
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

  fn apply(&self, ctx: &mut rspack_core::ApplyContext<'_>) -> Result<()> {
    ctx.compiler_hooks.compilation.tap(compilation::new(self));
    ctx
      .normal_module_factory_hooks
      .parser
      .tap(nmf_parser::new(self));
    Ok(())
  }
}
