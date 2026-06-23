use napi::Either;
use napi_derive::napi;

use crate::{asset::AssetInfo, chunk::ChunkWrapper};

#[napi(object)]
pub struct JsPathData {
  pub filename: Option<String>,
  pub hash: Option<String>,
  pub content_hash: Option<String>,
  pub runtime: Option<String>,
  pub url: Option<String>,
  pub id: Option<String>,
  pub chunk: Option<JsPathDataChunkLike>,
}

#[napi(object)]
pub struct JsPathDataChunkLike {
  pub name: Option<String>,
  pub hash: Option<String>,
  pub id: Option<String>,
}

/// Outbound `PathData` handed to a JS `filename` function.
///
/// Unlike [`JsPathData`] (which is also accepted *from* JS via
/// `compilation.getPath`), `chunk` here is a real `Chunk` instance whenever the
/// computation is associated with a chunk, so JS can call methods such as
/// `groupsIterable` / `getEntryOptions` on it. When no chunk instance is
/// available it falls back to the plain `{ name, id, hash }` shape, matching the
/// existing `Chunk | ChunkPathData` type.
#[napi(object, object_from_js = false)]
pub struct JsRenderPathData {
  pub filename: Option<String>,
  pub hash: Option<String>,
  pub content_hash: Option<String>,
  pub runtime: Option<String>,
  pub url: Option<String>,
  pub id: Option<String>,
  #[napi(ts_type = "Chunk | JsPathDataChunkLike")]
  pub chunk: Option<Either<ChunkWrapper, JsPathDataChunkLike>>,
}

impl JsRenderPathData {
  pub fn from_path_data(path_data: rspack_core::PathData) -> JsRenderPathData {
    let chunk = match (path_data.chunk_ukey, path_data.compilation) {
      // We have a live chunk + compilation: hand JS a real `Chunk` instance.
      (Some(chunk_ukey), Some(compilation)) => {
        Some(Either::A(ChunkWrapper::new(chunk_ukey, compilation)))
      }
      // Otherwise fall back to the plain chunk-like snapshot built from the
      // string fields, preserving the previous behavior for call sites that
      // don't carry a chunk ukey.
      _ if path_data.chunk_name.is_some()
        || path_data.chunk_id.is_some()
        || path_data.chunk_hash.is_some() =>
      {
        Some(Either::B(JsPathDataChunkLike {
          name: path_data.chunk_name.map(|s| s.to_string()),
          hash: path_data.chunk_hash.map(|s| s.to_string()),
          id: path_data.chunk_id.map(|s| s.to_string()),
        }))
      }
      _ => None,
    };

    JsRenderPathData {
      filename: path_data.filename.map(|s| s.to_string()),
      hash: path_data.hash.map(|s| s.to_string()),
      content_hash: path_data.content_hash.map(|s| s.to_string()),
      runtime: path_data.runtime.map(|s| s.to_string()),
      url: path_data.url.map(|s| s.to_string()),
      id: path_data.id.map(|s| s.to_string()),
      chunk,
    }
  }
}

impl JsPathData {
  pub fn to_path_data(&self) -> rspack_core::PathData<'_> {
    rspack_core::PathData {
      filename: self.filename.as_deref(),
      chunk_name: self.chunk.as_ref().and_then(|c| c.name.as_deref()),
      chunk_hash: self.chunk.as_ref().and_then(|c| c.hash.as_deref()),
      chunk_id: self.chunk.as_ref().and_then(|c| c.id.as_deref()),
      module_id: None,
      hash: self.hash.as_deref(),
      content_hash: self.content_hash.as_deref(),
      runtime: self.runtime.as_deref(),
      url: self.url.as_deref(),
      id: self.id.as_deref(),
      chunk_ukey: None,
      compilation: None,
    }
  }
}

#[napi(object)]
pub struct PathWithInfo {
  pub path: String,
  pub info: AssetInfo,
}

impl From<(String, rspack_core::AssetInfo)> for PathWithInfo {
  fn from(value: (String, rspack_core::AssetInfo)) -> Self {
    Self {
      path: value.0,
      info: value.1.into(),
    }
  }
}
